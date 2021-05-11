pub mod generated;
mod types;

pub use types::{Key, KeyPrefix, Value};

use crate::extensions::{rce_validator::generated::xudt_rce::SmtUpdate, Extension};
use crate::types::DeployedScriptConfig;

use anyhow::Result;
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed};
use molecule::prelude::Entity;

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

pub const RCE: &str = "rce";

pub struct RceValidatorExtension<S> {
    store: S,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> RceValidatorExtension<S> {
    pub fn new(store: S, config: HashMap<String, DeployedScriptConfig>) -> Self {
        Self { store, config }
    }
}

impl<S> Extension for RceValidatorExtension<S>
where
    S: Store,
{
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut rollback_insertions = HashSet::new();
        let mut rollback_deletions = HashSet::new();

        let mut batch = self.store.batch()?;
        for tx in block.data().transactions().into_iter() {
            for (idx, output) in tx.raw().outputs().into_iter().enumerate() {
                if self.is_rce_cell(&output) {
                    self.rce_process(
                        idx,
                        &mut batch,
                        &output.type_().to_opt().unwrap(),
                        &tx,
                        &mut rollback_deletions,
                        &mut rollback_insertions,
                    )?;
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

impl<S: Store> RceValidatorExtension<S> {
    fn rce_process(
        &self,
        index: usize,
        batch: &mut S::Batch,
        type_script: &packed::Script,
        tx: &packed::Transaction,
        rollback_deletions: &mut HashSet<Bytes>,
        rollback_insertions: &mut HashSet<Bytes>,
    ) -> Result<()> {
        // TODO: do we need to purge unused scripts?
        let script_hash = type_script.calc_script_hash();
        batch.put_kv(Key::ScriptHash(&script_hash), Value::Script(&type_script))?;

        let witness = tx.witnesses().get(index).expect("invalid witness");
        let witness_args =
            packed::WitnessArgs::from_slice(&witness.raw_data()).expect("invalid witness format");
        let output_type = witness_args
            .output_type()
            .to_opt()
            .expect("invalid witness output type");
        let smt_update =
            SmtUpdate::from_slice(&output_type.raw_data()).expect("invalid smt update");

        for item in smt_update.update().into_iter() {
            self.update_smt(
                &item,
                type_script,
                batch,
                rollback_deletions,
                rollback_insertions,
            )?;
        }

        Ok(())
    }

    fn update_smt(
        &self,
        item: &generated::xudt_rce::SmtUpdateItem,
        type_script: &packed::Script,
        batch: &mut S::Batch,
        rollback_deletions: &mut HashSet<Bytes>,
        rollback_insertions: &mut HashSet<Bytes>,
    ) -> Result<(), anyhow::Error> {
        let presence = u8::from(item.values()) & 0xF == 0x1;
        let address = item.key();
        let key = Key::Address(&type_script.calc_script_hash(), &address).into_vec();
        let old_presence = self.store.exists(key.clone())?;

        if presence {
            batch.put_kv(key.clone(), vec![0x1])?;
            if !old_presence {
                rollback_deletions.insert(Bytes::from(key));
            }
        } else {
            batch.delete(&key)?;
            if old_presence {
                rollback_insertions.insert(Bytes::from(key));
            }
        }

        Ok(())
    }

    fn is_rce_cell(&self, cell: &packed::CellOutput) -> bool {
        if let Some(type_script) = cell.type_().to_opt() {
            let rce_config = self.config.get(RCE).expect("empty config");

            type_script.code_hash() == rce_config.script.code_hash()
                && type_script.hash_type() == rce_config.script.hash_type()
        } else {
            false
        }
    }
}
