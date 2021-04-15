mod generated;

use crate::{
    error::Error,
    extensions::{rce_validator::generated::xudt_rce::SmtUpdate, Extension},
    types::DeployedScriptConfig,
};
use anyhow::Result;
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::{
    bytes::Bytes,
    core::{BlockNumber, BlockView},
    packed::{Byte32, WitnessArgs},
};
use molecule::prelude::Entity;
use std::collections::HashSet;
use std::convert::TryInto;

pub struct RceValidatorExtension<S> {
    store: S,
    config: DeployedScriptConfig,
}

impl<S> RceValidatorExtension<S> {
    pub fn new(store: S, config: DeployedScriptConfig) -> Self {
        Self { store, config }
    }

    // pub fn store(&self) -> &S {
    //     &self.store
    // }
}

pub enum Key<'a> {
    Address(&'a Bytes, &'a Byte32),
    Block(BlockNumber, &'a Byte32),
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
}

impl<'a> Key<'a> {
    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::Address(script_args, key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(&script_args);
                encoded.extend_from_slice(key.as_slice());
            }
            Key::Block(block_number, block_hash) => {
                encoded.push(KeyPrefix::Block as u8);
                encoded.extend_from_slice(&block_number.to_be_bytes());
                encoded.extend_from_slice(block_hash.as_slice());
            }
        }

        encoded
    }
}

pub enum Value {
    RollbackData(Vec<Bytes>, Vec<Bytes>),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();
        match self {
            Value::RollbackData(insertions, deletions) => {
                encoded.extend_from_slice(&(insertions.len() as u64).to_be_bytes());
                insertions.iter().for_each(|key| {
                    encoded.extend_from_slice(&(key.len() as u64).to_be_bytes());
                    encoded.extend_from_slice(&key);
                });
                encoded.extend_from_slice(&(deletions.len() as u64).to_be_bytes());
                deletions.iter().for_each(|key| {
                    encoded.extend_from_slice(&(key.len() as u64).to_be_bytes());
                    encoded.extend_from_slice(&key);
                });
            }
        }
        encoded
    }
}

impl Value {
    pub fn parse_data(slice: &[u8]) -> (Vec<Bytes>, Vec<Bytes>) {
        let mut offset = 0;
        let mut insertions = vec![];
        let insertion_count = u64::from_be_bytes(
            slice[offset..offset + 8]
                .try_into()
                .expect("insertion count"),
        ) as usize;
        offset += 8;
        for _ in 0..insertion_count {
            let len = u64::from_be_bytes(
                slice[offset..offset + 8]
                    .try_into()
                    .expect("insertion length"),
            ) as usize;
            offset += 8;
            insertions.push(Bytes::from(slice[offset..(offset + len)].to_vec()));
            offset += len;
        }
        let mut deletions = vec![];
        let deletion_count = u64::from_be_bytes(
            slice[offset..offset + 8]
                .try_into()
                .expect("deletion count"),
        ) as usize;
        offset += 8;
        for _ in 0..deletion_count {
            let len = u64::from_be_bytes(
                slice[offset..offset + 8]
                    .try_into()
                    .expect("deletion length"),
            ) as usize;
            offset += 8;
            deletions.push(Bytes::from(slice[offset..(offset + len)].to_vec()));
            offset += len;
        }
        assert!(offset == slice.len());
        (insertions, deletions)
    }
}

impl<S> Extension for RceValidatorExtension<S>
where
    S: Store,
{
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut rollback_insertions = HashSet::new();
        let mut rollback_deletions = HashSet::new();

        let mut batch = self.store.batch().map_err(|e| Error::from(e))?;
        for tx in block.data().transactions() {
            for (i, output) in tx.raw().outputs().into_iter().enumerate() {
                if let Some(type_script) = output.type_().to_opt() {
                    if type_script.code_hash() == self.config.script.code_hash()
                        && type_script.hash_type() == self.config.script.hash_type()
                    {
                        let witness = tx.witnesses().get(i).expect("invalid witness");
                        let witness_args = WitnessArgs::from_slice(&witness.raw_data())
                            .expect("invalid witness format");
                        let output_type = witness_args
                            .output_type()
                            .to_opt()
                            .expect("invalid witness output type");
                        let smt_update = SmtUpdate::from_slice(&output_type.raw_data())
                            .expect("invalid smt update");

                        for item in smt_update.update() {
                            let presence = u8::from(item.values()) & 0xF == 0x1;
                            let script_args = type_script.args().raw_data();
                            let address = item.key();
                            let key = Key::Address(&script_args, &address).into_vec();
                            let old_presence =
                                self.store.exists(key.clone()).map_err(|e| Error::from(e))?;
                            if presence {
                                batch
                                    .put_kv(key.clone(), vec![0x1])
                                    .map_err(|e| Error::from(e))?;
                                if !old_presence {
                                    rollback_deletions.insert(Bytes::from(key));
                                }
                            } else {
                                batch.delete(key.clone()).map_err(|e| Error::from(e))?;
                                if old_presence {
                                    rollback_insertions.insert(Bytes::from(key));
                                }
                            }
                        }
                    }
                }
            }
        }
        let block_hash = block.hash();
        batch
            .put_kv(
                Key::Block(block.number(), &block_hash),
                Value::RollbackData(
                    rollback_insertions.into_iter().collect(),
                    rollback_deletions.into_iter().collect(),
                ),
            )
            .map_err(|e| Error::from(e))?;
        batch.commit().map_err(|e| Error::from(e))?;
        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, &tip_hash).into_vec();
        let data = self
            .store
            .get(block_key.clone())
            .map_err(|e| Error::from(e))?
            .expect("rollback data do not exist!");
        let (insertions, deletions) = Value::parse_data(&data);

        let mut batch = self.store.batch().map_err(|e| Error::from(e))?;
        for insertion_key in &insertions {
            batch
                .put(insertion_key, vec![0x1])
                .map_err(|e| Error::from(e))?;
        }
        for deletion_key in &deletions {
            batch.delete(deletion_key).map_err(|e| Error::from(e))?;
        }
        batch.delete(block_key).map_err(|e| Error::from(e))?;
        batch.commit().map_err(|e| Error::from(e))?;
        Ok(())
    }

    fn prune(&self, tip_number: BlockNumber, _tip_hash: &Byte32, keep_num: u64) -> Result<()> {
        if tip_number > keep_num {
            let prune_to_block = tip_number - keep_num;
            let mut batch = self.store.batch().map_err(|e| Error::from(e))?;
            let block_key_prefix = vec![KeyPrefix::Block as u8];
            let iter = self
                .store
                .iter(&block_key_prefix, IteratorDirection::Forward)
                .map_err(|e| Error::from(e))?
                .take_while(|(key, _value)| key.starts_with(&block_key_prefix));
            for (_block_number, key) in iter
                .map(|(key, _value)| {
                    (
                        BlockNumber::from_be_bytes(
                            key[1..9].try_into().expect("stored block_number"),
                        ),
                        key,
                    )
                })
                .take_while(|(block_number, _key)| prune_to_block.gt(block_number))
            {
                batch.delete(key).map_err(|e| Error::from(e))?;
            }
            batch.commit().map_err(|e| Error::from(e))?;
        }
        Ok(())
    }
}
