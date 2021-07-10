pub mod types;

pub use types::{Key, KeyPrefix, SpMap, SpecialCellKind, SpecialCellsExtensionError, Value};

use crate::{types::DeployedScriptConfig, DetailedCell, DetailedCells, Extension};

use common::utils::{find, remove_item, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160, AddressPayload, CodeHashIndex, NetworkType};

use bincode::{deserialize, serialize};
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
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
        let mut sp_map = SpMap::default();
        let block_number = block.number();
        let block_hash = block.hash();
        let epoch_number = block.epoch().to_rational().into_u256();

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

                if self.is_sp_cell(ACP, &cell.cell_output) {
                    let detail_cell = DetailedCell::from_detailed_live_cell(
                        epoch_number.clone(),
                        cell,
                        out_point.clone(),
                    );
                    let key = self.get_acp_pubkey_hash(&detail_cell.cell_output.lock().args());
                    sp_map.entry_and_push_remove(key, detail_cell);
                } else if self.is_sp_cell(CHEQUE, &cell.cell_output) {
                    let detail_cell = DetailedCell::from_detailed_live_cell(
                        epoch_number.clone(),
                        cell,
                        out_point.clone(),
                    );
                    let (sender, receiver) =
                        self.get_sender_and_receiver(&detail_cell.cell_output.lock().args());

                    sp_map.entry_and_push_remove(sender, detail_cell.clone());
                    sp_map.entry_and_push_remove(receiver, detail_cell);
                }
            }

            for (idx, output) in tx.outputs().into_iter().enumerate() {
                if self.is_sp_cell(ACP, &output) {
                    let key = self.get_acp_pubkey_hash(&output.lock().args());
                    let (_, cell_data) = tx.output_with_data(idx).unwrap();
                    let detail_cell = DetailedCell::new(
                        epoch_number.clone(),
                        block_number,
                        block_hash.clone(),
                        output,
                        tx.hash(),
                        (idx as u32).pack(),
                        cell_data.pack(),
                    );

                    sp_map.entry_and_push_add(key, detail_cell);
                } else if self.is_sp_cell(CHEQUE, &output) {
                    let (sender, receiver) = self.get_sender_and_receiver(&output.lock().args());
                    let (_, cell_data) = tx.output_with_data(idx).unwrap();
                    let detail_cell = DetailedCell::new(
                        epoch_number.clone(),
                        block_number,
                        block_hash.clone(),
                        output,
                        tx.hash(),
                        (idx as u32).pack(),
                        cell_data.pack(),
                    );

                    sp_map.entry_and_push_add(sender, detail_cell.clone());
                    sp_map.entry_and_push_add(receiver, detail_cell);
                }
            }
        }

        self.store_sp_cells(sp_map, block.number(), &block.hash(), false)?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let raw_data = self
            .store
            .get(&block_key)?
            .expect("Special Cells extension rollback data is not exist");

        let sp_map = deserialize::<SpMap>(&raw_data).unwrap();
        self.store_sp_cells(sp_map, tip_number, &tip_hash, true)?;
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

    fn is_sp_cell(&self, cell_name: &str, cell_output: &packed::CellOutput) -> bool {
        let script = cell_output.lock();
        let config = self
            .config
            .get(cell_name)
            .unwrap_or_else(|| panic!("Special Cell extension config is empty"));

        if script.code_hash() == config.script.code_hash()
            && script.hash_type() == config.script.hash_type()
        {
            return true;
        }

        false
    }

    fn get_acp_pubkey_hash(&self, lock_args: &packed::Bytes) -> H160 {
        let tmp: Vec<u8> = lock_args.unpack();
        let pubkey_hash = H160::from_slice(&tmp[0..20]).unwrap();
        let script = packed::Script::from(&AddressPayload::new_short(
            CodeHashIndex::Sighash,
            pubkey_hash,
        ));

        H160::from_slice(&blake2b_160(script.as_slice())).unwrap()
    }

    fn get_sender_and_receiver(&self, lock_args: &packed::Bytes) -> (H160, H160) {
        let bytes: Vec<u8> = lock_args.unpack();
        let receiver = H160::from_slice(&bytes[0..20]).unwrap();
        let sender = H160::from_slice(&bytes[20..40]).unwrap();
        (sender, receiver)
    }

    fn store_sp_cells(
        &self,
        sp_map: SpMap,
        block_num: BlockNumber,
        block_hash: &packed::Byte32,
        is_reverse: bool,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (key, mut val) in sp_map.0.clone().into_iter() {
            val.remove_intersection();

            if is_reverse {
                val.reverse();
            }

            let addr_key = Key::CkbAddress(&key).into_vec();
            let mut cells = self
                .store
                .get(&addr_key)?
                .map_or_else(DetailedCells::default, |bytes| deserialize(&bytes).unwrap());

            for removed in val.removed.0.into_iter() {
                if !cells.contains(&removed) {
                    return Err(SpecialCellsExtensionError::MissingSPCell {
                        cell_kind: self.sp_cell_kind(&removed.cell_output.lock().code_hash()),
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
            Key::Block(block_num, block_hash).into_vec(),
            Value::RollbackData(serialize(&sp_map).unwrap()),
        )?;
        batch.commit()?;

        Ok(())
    }

    fn sp_cell_kind(&self, lock_code_hash: &packed::Byte32) -> SpecialCellKind {
        if &self.config.get(ACP).unwrap().script.code_hash() == lock_code_hash {
            SpecialCellKind::AnyoneCanPay
        } else {
            SpecialCellKind::ChequeDeposit
        }
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }
}
