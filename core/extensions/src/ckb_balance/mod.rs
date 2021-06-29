mod types;

pub use types::{
    Balance, BalanceDelta, CkbBalanceExtensionError, CkbBalanceMap, Key, KeyPrefix, Value,
};

use crate::{types::DeployedScriptConfig, Extension};

use common::anyhow::Result;
use common::utils::{find, to_fixed_array};

use bincode::deserialize;
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::Unpack};
use common::NetworkType;

use std::collections::HashMap;
use std::sync::Arc;

pub const SECP256K1_BLAKE160: &str = "secp256k1_blake160";
const SUDT: &str = "sudt_balance";

pub struct CkbBalanceExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for CkbBalanceExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut ckb_balance_map = CkbBalanceMap::default();
        let mut ckb_balance_change = ckb_balance_map.inner_mut();

        for (index, tx) in block.transactions().iter().enumerate() {
            if index > 0 {
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

                    if is_secp256k1_blake160_cell(&cell.cell_output, &self.config) {
                        self.change_ckb_balance_normal_cell_capacity(
                            &cell.cell_output,
                            &mut ckb_balance_change,
                            true,
                        );
                    }

                    if is_secp256k1_blake160_udt_cell(&cell.cell_output, &self.config) {
                        self.change_ckb_balance_udt_cell_capacity(
                            &cell.cell_output,
                            &mut ckb_balance_change,
                            true,
                        );
                    }
                }
            }

            for output in tx.outputs().into_iter() {
                if is_secp256k1_blake160_cell(&output, &self.config) {
                    self.change_ckb_balance_normal_cell_capacity(
                        &output,
                        &mut ckb_balance_change,
                        false,
                    );
                }

                if is_secp256k1_blake160_udt_cell(&output, &self.config) {
                    self.change_ckb_balance_udt_cell_capacity(
                        &output,
                        &mut ckb_balance_change,
                        false,
                    );
                }
            }
        }

        self.store_balance(block.number(), &block.hash(), ckb_balance_map)?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let map = self
            .store
            .get(block_key)?
            .expect("CKB extension rollback data does not exist");

        let mut delta_map = deserialize::<CkbBalanceMap>(&map).unwrap();
        delta_map.opposite_value();

        self.store_balance(tip_number, tip_hash, delta_map)?;

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

impl<S: Store, BS: Store> CkbBalanceExtension<S, BS> {
    pub fn new(
        store: S,
        indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        CkbBalanceExtension {
            store,
            indexer,
            _net_ty,
            config,
        }
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or_else(|| {
                let tx_hash: [u8; 32] = out_point.tx_hash().unpack();
                CkbBalanceExtensionError::NoLiveCellByOutpoint {
                    tx_hash: hex::encode(tx_hash),
                    index: out_point.index().unpack(),
                }
                .into()
            })
    }

    fn get_cell_capacity(&self, cell_output: &packed::CellOutput) -> u64 {
        cell_output.capacity().unpack()
    }

    fn change_ckb_balance_normal_cell_capacity(
        &self,
        cell_output: &packed::CellOutput,
        ckb_balance_map: &mut HashMap<[u8; 32], BalanceDelta>,
        is_sub: bool,
    ) {
        let addr: [u8; 32] = cell_output.lock().calc_script_hash().unpack();
        let capacity = self.get_cell_capacity(&cell_output);

        if is_sub {
            ckb_balance_map
                .entry(addr)
                .or_insert_with(BalanceDelta::default)
                .normal_cell_capacity -= capacity as i128;
        } else {
            ckb_balance_map
                .entry(addr)
                .or_insert_with(BalanceDelta::default)
                .normal_cell_capacity += capacity as i128;
        }
    }

    fn change_ckb_balance_udt_cell_capacity(
        &self,
        cell_output: &packed::CellOutput,
        ckb_balance_map: &mut HashMap<[u8; 32], BalanceDelta>,
        is_sub: bool,
    ) {
        let addr: [u8; 32] = cell_output.lock().calc_script_hash().unpack();
        let capacity = self.get_cell_capacity(&cell_output);

        if is_sub {
            ckb_balance_map
                .entry(addr)
                .or_insert_with(BalanceDelta::default)
                .udt_cell_capacity -= capacity as i128;
        } else {
            ckb_balance_map
                .entry(addr)
                .or_insert_with(BalanceDelta::default)
                .udt_cell_capacity += capacity as i128;
        }
    }

    fn store_balance(
        &self,
        block_num: BlockNumber,
        block_hash: &packed::Byte32,
        ckb_balance_map: CkbBalanceMap,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (addr, balance_delta) in ckb_balance_map.inner().iter() {
            let key = Key::CkbAddress(addr).into_vec();
            let original_balance = self
                .store
                .get(&key)?
                .map_or_else(Balance::default, |bytes| deserialize(&bytes).unwrap());

            let current_normal_cell_capacity = add(
                original_balance.normal_cell_capacity,
                balance_delta.normal_cell_capacity,
            );

            let current_udt_cell_capacity = add(
                original_balance.udt_cell_capacity,
                balance_delta.udt_cell_capacity,
            );

            let current_balance = Balance::new(
                current_normal_cell_capacity as u64,
                current_udt_cell_capacity as u64,
            );

            if current_normal_cell_capacity < 0 || current_udt_cell_capacity < 0 {
                return Err(CkbBalanceExtensionError::BalanceIsNegative(
                    hex::encode(&addr),
                    current_balance,
                )
                .into());
            } else {
                batch.put_kv(key, Value::CkbBalance(current_balance))?;
            }
        }

        batch.put_kv(
            Key::Block(block_num, &block_hash),
            Value::RollbackData(ckb_balance_map),
        )?;

        batch.commit()?;

        Ok(())
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn get_balance(&self, addr: &str) -> Result<Option<u64>> {
        let script: packed::Script = common::utils::parse_address(addr)?.payload().into();
        let hash: [u8; 32] = script.calc_script_hash().unpack();
        let bytes = Key::CkbAddress(&hash).into_vec();
        self.store
            .get(bytes)
            .map(|tmp| tmp.map(|bytes| u64::from_be_bytes(to_fixed_array(&bytes))))
            .map_err(Into::into)
    }
}

fn add(a: u64, b: i128) -> i128 {
    (a as i128) + b
}

fn is_secp256k1_blake160_cell(
    cell: &packed::CellOutput,
    config: &HashMap<String, DeployedScriptConfig>,
) -> bool {
    let lock_script = &config.get(SECP256K1_BLAKE160).unwrap().script;
    cell.lock().code_hash() == lock_script.code_hash()
        && cell.lock().hash_type() == lock_script.hash_type()
        && cell.type_().is_none()
}

fn is_secp256k1_blake160_udt_cell(
    cell: &packed::CellOutput,
    config: &HashMap<String, DeployedScriptConfig>,
) -> bool {
    let lock_script = &config.get(SECP256K1_BLAKE160).unwrap().script;
    let sudt_script = &config.get(SUDT).unwrap().script;
    // let xudt_script = &config.get(UDTType::Extensible.as_str()).unwrap().script;
    let check_lock_script = cell.lock().code_hash() == lock_script.code_hash()
        && cell.lock().hash_type() == lock_script.hash_type();
    if check_lock_script && cell.type_().is_some() {
        let type_script = cell.type_().to_opt().unwrap();
        type_script.code_hash() == sudt_script.code_hash()
            && type_script.hash_type() == sudt_script.hash_type()
        // || type_script.code_hash() == xudt_script.code_hash()
        // && type_script.hash_type() == xudt_script.hash_type()
    } else {
        false
    }
}
