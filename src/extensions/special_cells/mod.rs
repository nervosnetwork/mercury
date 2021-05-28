pub mod types;

use types::{ACPMap, Key, KeyPrefix, SpecialCellsExtensionError, Value};

use crate::extensions::{DetailedCell, DetailedCells, Extension};
use crate::types::DeployedScriptConfig;
use crate::utils::{find, remove_item, to_fixed_array};

use anyhow::Result;
use bincode::{deserialize, serialize};
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*, H160};

use std::collections::HashMap;
use std::sync::Arc;

pub const ACP: &str = "anyone_can_pay";
pub const CHEQUE: &str = "cheque";

pub struct SpecialCellsExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for SpecialCellsExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        if block.is_genesis() {
            return Ok(());
        }

        let mut acp_map = ACPMap::default();
        let block_number = block.number();
        let block_hash = block.hash();

        for tx in block.transactions().iter().skip(1) {
            for input in tx.inputs().into_iter() {
                let out_point = input.previous_output();
                let tx_hash = out_point.tx_hash();
                let cell = if block.tx_hashes().contains(&tx_hash) {
                    let tx_index = find(&tx_hash, block.tx_hashes()).unwrap();
                    let tx = block.transactions().get(tx_index).cloned().unwrap();
                    let cell_index: u32 = out_point.index().unpack();
                    let cell = tx.outputs().get(cell_index as usize).unwrap();
                    let data = tx.outputs_data().get(cell_index as usize).unwrap();

                    DetailedLiveCell {
                        block_number: block.number(),
                        block_hash: block.hash(),
                        tx_index: tx_index as u32,
                        cell_output: cell,
                        cell_data: data,
                    }
                } else {
                    self.get_live_cell_by_out_point(&out_point)?
                };

                if self.is_acp_cell(&cell.cell_output) {
                    let detail_cell =
                        DetailedCell::from_detailed_live_cell(cell, out_point.clone());
                    let key = self.get_acp_pubkey_hash(&detail_cell.cell_output.lock().args());
                    acp_map
                        .0
                        .entry(key)
                        .or_insert_with(Default::default)
                        .push_removed(detail_cell);
                }
            }

            for (idx, output) in tx.outputs().into_iter().enumerate() {
                if self.is_acp_cell(&output) {
                    let key = self.get_acp_pubkey_hash(&output.lock().args());
                    let (_, cell_data) = tx.output_with_data(idx).unwrap();
                    let detail_cell = DetailedCell::new(
                        block_number,
                        block_hash.clone(),
                        output,
                        tx.hash(),
                        (idx as u32).pack(),
                        cell_data.pack(),
                    );

                    acp_map
                        .0
                        .entry(key)
                        .or_insert_with(Default::default)
                        .push_added(detail_cell);
                }
            }
        }

        self.store_acp_cells(acp_map, block.number(), &block.hash(), false)?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let raw_data = self
            .store
            .get(&block_key)?
            .expect("ACP extension rollback data is not exist");

        let acp_map = deserialize::<ACPMap>(&raw_data).unwrap();
        self.store_acp_cells(acp_map, tip_number, &tip_hash, true)?;
        Ok(())
    }

    fn prune(
        &self,
        tip_number: BlockNumber,
        _tip_hash: &packed::Byte32,
        keep_num: u64,
    ) -> Result<()> {
        let prune_to_block = tip_number - keep_num;
        let block_key_prefix = vec![KeyPrefix::Block as u8];
        let mut batch = self.get_batch()?;

        let block_iter = self
            .store
            .iter(&block_key_prefix, IteratorDirection::Forward)?
            .filter(|(key, _v)| {
                key.starts_with(&block_key_prefix)
                    && BlockNumber::from_be_bytes(to_fixed_array(&key[1..9])) < prune_to_block
            });

        for (key, _val) in block_iter {
            batch.delete(key)?;
        }

        batch.commit()?;

        Ok(())
    }
}

impl<S: Store, BS: Store> SpecialCellsExtension<S, BS> {
    pub fn new(
        store: S,
        indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        SpecialCellsExtension {
            store,
            indexer,
            _net_ty,
            config,
        }
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or_else(|| {
                SpecialCellsExtensionError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                }
                .into()
            })
    }

    fn is_acp_cell(&self, cell_output: &packed::CellOutput) -> bool {
        let script = cell_output.lock();
        let config = self
            .config
            .get(ACP)
            .unwrap_or_else(|| panic!("ACP extension config is empty"));

        if script.code_hash() == config.script.code_hash()
            && script.hash_type() == config.script.hash_type()
        {
            return true;
        }

        false
    }

    fn get_acp_pubkey_hash(&self, lock_args: &packed::Bytes) -> H160 {
        let hash: Vec<u8> = lock_args.unpack();
        H160::from_slice(&hash[0..20]).unwrap()
    }

    fn store_acp_cells(
        &self,
        acp_map: ACPMap,
        block_num: BlockNumber,
        block_hash: &packed::Byte32,
        is_reverse: bool,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (key, mut val) in acp_map.0.clone().into_iter() {
            val.remove_intersection();

            if is_reverse {
                val.reverse();
            }

            let addr_key = Key::CkbAddress(&key);
            let mut cells = self
                .store
                .get(&addr_key.clone().into_vec())?
                .map_or_else(DetailedCells::default, |bytes| deserialize(&bytes).unwrap());

            for removed in val.removed.0.into_iter() {
                if !cells.contains(&removed) {
                    return Err(SpecialCellsExtensionError::MissingSPCell {
                        tx_hash: hex::encode(removed.out_point.tx_hash().as_slice()),
                        index: removed.out_point.index().unpack(),
                    }
                    .into());
                }

                remove_item(&mut cells.0, &removed);
            }

            for added in val.added.0.into_iter() {
                cells.push(added);
            }

            batch.put_kv(addr_key, Value::SPCells(cells))?;
        }

        batch.put_kv(
            Key::Block(block_num, block_hash),
            Value::RollbackData(serialize(&acp_map).unwrap()),
        )?;
        batch.commit()?;

        Ok(())
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }
}
