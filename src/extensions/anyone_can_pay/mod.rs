pub mod types;

use types::{ACPExtensionError, ACPMap, Key, KeyPrefix, Value};

use crate::extensions::Extension;
use crate::types::DeployedScriptConfig;
use crate::utils::{find, remove_item, to_fixed_array};

use anyhow::Result;
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*, H160};

use std::collections::HashMap;
use std::sync::Arc;

const ACP: &str = "anyone_can_pay";

pub struct ACPExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for ACPExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        if block.is_genesis() {
            return Ok(());
        }

        let mut acp_map = ACPMap::default();

        for (idx, tx) in block.transactions().iter().enumerate() {
            if idx > 0 {
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
                        let key = self.get_acp_pubkey_hash(&cell.cell_output.lock().args());
                        acp_map
                            .0
                            .entry(key)
                            .or_insert_with(Default::default)
                            .push_removed(&out_point);
                    }
                }
            }

            for (idx, output) in tx.outputs().into_iter().enumerate() {
                if self.is_acp_cell(&output) {
                    let key = self.get_acp_pubkey_hash(&output.lock().args());
                    let out_point = packed::OutPointBuilder::default()
                        .tx_hash(tx.hash())
                        .index((idx as u32).pack())
                        .build();

                    acp_map
                        .0
                        .entry(key)
                        .or_insert_with(Default::default)
                        .push_added(out_point);
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

        let acp_map = rlp::decode::<ACPMap>(&raw_data).unwrap();
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

impl<S: Store, BS: Store> ACPExtension<S, BS> {
    pub fn new(
        store: S,
        indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        ACPExtension {
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
                ACPExtensionError::CannotGetLiveCellByOutPoint {
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
            let mut out_points =
                self.store
                    .get(&addr_key.clone().into_vec())?
                    .map_or_else(Vec::new, |bytes| {
                        packed::OutPointVec::from_slice(&bytes)
                            .unwrap()
                            .into_iter()
                            .collect()
                    });

            for removed in val.removed.into_iter() {
                if !out_points.contains(&removed) {
                    return Err(ACPExtensionError::MissingACPCell {
                        tx_hash: hex::encode(removed.tx_hash().as_slice()),
                        index: removed.index().unpack(),
                    }
                    .into());
                }

                remove_item(&mut out_points, &removed);
            }

            for added in val.added.into_iter() {
                out_points.push(added);
            }

            batch.put_kv(addr_key, Value::ACPCells(out_points.pack()))?;
        }

        batch.put_kv(
            Key::Block(block_num, block_hash),
            Value::RollbackData(rlp::encode(&acp_map)),
        )?;
        batch.commit()?;

        Ok(())
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }
}
