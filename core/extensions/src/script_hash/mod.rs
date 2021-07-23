pub mod types;

pub use types::{Key, KeyPrefix, ScriptHashExtensionError, Value};

use crate::{types::DeployedScriptConfig, Extension};

use common::{anyhow::Result, hash::blake2b_160, NetworkType};
use core_storage::{Batch, Store};

use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*};

use std::collections::HashMap;
use std::sync::Arc;

pub struct ScriptHashExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    _config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for ScriptHashExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let block_num = block.number();
        let block_hash = block.hash();
        let mut batch = self.store.batch()?;

        for (tx_index, tx) in block.transactions().iter().enumerate() {
            let tx_hash: [u8; 32] = tx.hash().unpack();
            batch.put_kv(
                Key::TxHash(tx_hash),
                Value::BlockNumAndHash(block_num, &block_hash),
            )?;

            if tx_index > 0 {
                for (io_index, input) in tx.inputs().into_iter().enumerate() {
                    let out_point = input.previous_output();
                    let cell_index: u32 = out_point.index().unpack();
                    let cell = if block.tx_hashes().contains(&out_point.tx_hash()) {
                        tx.outputs().get(cell_index as usize).unwrap()
                    } else {
                        let detailed_cell = self.get_live_cell_by_out_point(&out_point)?;
                        detailed_cell.cell_output
                    };
                    let type_hash = if let Some(script) = cell.type_().to_opt() {
                        script.calc_script_hash()
                    } else {
                        packed::Byte32::zero()
                    };

                    batch.put_kv(
                        Key::CellTypeHash(tx_hash, io_index as u32, 0),
                        Value::TypeHash(&type_hash),
                    )?;
                }
            }

            for (io_index, output) in tx.outputs().into_iter().enumerate() {
                let lock_hash = blake2b_160(output.lock().as_slice());
                batch.put_kv(Key::ScriptHash(lock_hash), Value::Script(&output.lock()))?;

                let type_hash = if let Some(script) = output.type_().to_opt() {
                    script.calc_script_hash()
                } else {
                    packed::Byte32::zero()
                };

                batch.put_kv(
                    Key::CellTypeHash(tx_hash, io_index as u32, 1),
                    Value::TypeHash(&type_hash),
                )?;
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
        indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        _config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        ScriptHashExtension {
            store,
            indexer,
            _net_ty,
            _config,
        }
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or_else(|| {
                ScriptHashExtensionError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                }
                .into()
            })
    }
}
