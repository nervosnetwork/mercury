pub mod types;

pub use types::{Key, KeyPrefix, ScriptHashExtensionError, Value};

use crate::{types::DeployedScriptConfig, Extension};

use common::{anyhow::Result, hash::blake2b_160, NetworkType};
use core_storage::{Batch, Store};

use ckb_indexer::indexer::Indexer;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*};

use std::collections::HashMap;
use std::sync::Arc;

pub struct ScriptHashExtension<S, BS> {
    store: S,
    _indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    _config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for ScriptHashExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let block_num = block.number();
        let block_hash = block.hash();
        let mut batch = self.store.batch()?;

        for tx in block.transactions().iter() {
            let tx_hash: [u8; 32] = tx.hash().unpack();
            batch.put_kv(
                Key::TxHash(tx_hash),
                Value::BlockNumAndHash(block_num, &block_hash),
            )?;

            for output in tx.outputs().into_iter() {
                let lock_hash = blake2b_160(output.lock().as_slice());
                batch.put_kv(Key::ScriptHash(lock_hash), Value::Script(&output.lock()))?;
            }
        }

        batch.commit()?;
        Ok(())
    }

    fn rollback(&self, _tip_number: BlockNumber, _tip_hash: &packed::Byte32) -> Result<()> {
        Ok(())
    }

    fn prune(
        &self,
        _tip_number: BlockNumber,
        _tip_hash: &packed::Byte32,
        _keep_num: u64,
    ) -> Result<()> {
        Ok(())
    }
}

impl<S: Store, BS: Store> ScriptHashExtension<S, BS> {
    pub fn new(
        store: S,
        _indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        _config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        ScriptHashExtension {
            store,
            _indexer,
            _net_ty,
            _config,
        }
    }
}
