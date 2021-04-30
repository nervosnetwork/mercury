mod generated;

use crate::extensions::{rce_validator::generated::xudt_rce::SmtUpdate, Extension};
use crate::types::DeployedScriptConfig;

use anyhow::Result;
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed};
use molecule::prelude::Entity;

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

const RCE: &str = "rce";

pub struct RceValidatorExtension<S> {
    store: S,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> RceValidatorExtension<S> {
    pub fn new(store: S, config: HashMap<String, DeployedScriptConfig>) -> Self {
        Self { store, config }
    }
}

pub enum Key<'a> {
    Address(&'a Bytes, &'a packed::Byte32),
    Block(BlockNumber, &'a packed::Byte32),
    ScriptHash(&'a packed::Byte32),
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
    ScriptHash = 32,
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

            Key::ScriptHash(hash) => {
                encoded.push(KeyPrefix::ScriptHash as u8);
                encoded.extend_from_slice(hash.as_slice());
            }
        }

        encoded
    }
}

pub enum Value<'a> {
    RollbackData(Vec<Bytes>, Vec<Bytes>),
    Script(&'a packed::Script),
}

impl<'a> Into<Vec<u8>> for Value<'a> {
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

            Value::Script(script) => {
                encoded.extend_from_slice(script.as_slice());
            }
        }
        encoded
    }
}

impl<'a> Value<'a> {
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

    // pub fn parse_script(slice: &[u8]) -> Script {
    //     Script::from_slice(slice).expect("script parsing")
    // }
}

impl<S> Extension for RceValidatorExtension<S>
where
    S: Store,
{
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut rollback_insertions = HashSet::new();
        let mut rollback_deletions = HashSet::new();

        let mut batch = self.store.batch()?;
        for tx in block.data().transactions() {
            for (i, output) in tx.raw().outputs().into_iter().enumerate() {
                if let Some(type_script) = output.type_().to_opt() {
                    let rce_config = self.config.get(RCE).expect("empty config");

                    if type_script.code_hash() == rce_config.script.code_hash()
                        && type_script.hash_type() == rce_config.script.hash_type()
                    {
                        // TODO: do we need to purge unused scripts?
                        let script_hash = type_script.calc_script_hash();
                        batch.put_kv(Key::ScriptHash(&script_hash), Value::Script(&type_script))?;

                        let witness = tx.witnesses().get(i).expect("invalid witness");
                        let witness_args = packed::WitnessArgs::from_slice(&witness.raw_data())
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
                            let old_presence = self.store.exists(key.clone())?;
                            if presence {
                                batch.put_kv(key.clone(), vec![0x1])?;
                                if !old_presence {
                                    rollback_deletions.insert(Bytes::from(key));
                                }
                            } else {
                                batch.delete(key.clone())?;
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
        batch.put_kv(
            Key::Block(block.number(), &block_hash),
            Value::RollbackData(
                rollback_insertions.into_iter().collect(),
                rollback_deletions.into_iter().collect(),
            ),
        )?;

        batch.commit()?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, &tip_hash).into_vec();
        let data = self
            .store
            .get(block_key.clone())?
            .expect("rollback data do not exist!");
        let (insertions, deletions) = Value::parse_data(&data);

        let mut batch = self.store.batch()?;
        for insertion_key in &insertions {
            batch.put(insertion_key, vec![0x1])?;
        }

        for deletion_key in &deletions {
            batch.delete(deletion_key)?;
        }

        batch.delete(block_key)?;
        batch.commit()?;
        Ok(())
    }

    fn prune(
        &self,
        tip_number: BlockNumber,
        _tip_hash: &packed::Byte32,
        keep_num: u64,
    ) -> Result<()> {
        if tip_number > keep_num {
            let prune_to_block = tip_number - keep_num;
            let mut batch = self.store.batch()?;
            let block_key_prefix = vec![KeyPrefix::Block as u8];
            let iter = self
                .store
                .iter(&block_key_prefix, IteratorDirection::Forward)?
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
                batch.delete(key)?;
            }
            batch.commit()?;
        }
        Ok(())
    }
}
