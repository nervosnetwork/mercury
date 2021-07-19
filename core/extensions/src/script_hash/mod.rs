pub mod types;

use ckb_types::prelude::Entity;
pub use types::{Key, KeyPrefix, ScriptHashExtensionError, Value};

use crate::{types::DeployedScriptConfig, Extension};

use common::{anyhow::Result, hash::blake2b_160, NetworkType};

use ckb_indexer::indexer::Indexer;
use ckb_indexer::store::{Batch, Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::packed;

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
        let mut batch = self.store.batch()?;

        for tx in block.transactions().iter() {
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
