mod types;

pub use types::{
    Key, KeyPrefix, UDTBalanceExtensionError, UDTBalanceMap, UDTBalanceMaps, UDTType, Value,
};

use crate::{types::DeployedScriptConfig, Extension};

use common::anyhow::{format_err, Result};
use common::utils::{find, to_fixed_array};
use common::NetworkType;

use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::BlockView;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Unpack, H256};
use num_bigint::BigInt;
use num_traits::identities::Zero;
use rlp::{Decodable, Encodable, Rlp};

use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;

const UDT_AMOUNT_LEN: usize = 16;
pub const SUDT: &str = "sudt_balance";
pub const XUDT: &str = "xudt_balance";

pub struct UDTBalanceExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for UDTBalanceExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut sudt_balance_map = UDTBalanceMap::default();
        let mut sudt_balance_change = sudt_balance_map.inner_mut();
        let mut sudt_script_map = HashMap::new();

        let mut xudt_balance_map = UDTBalanceMap::default();
        let mut xudt_balance_change = xudt_balance_map.inner_mut();
        let mut xudt_script_map = HashMap::new();

        for (idx, tx) in block.transactions().iter().enumerate() {
            // Skip cellbase.
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

                    if cell.cell_data.raw_data().len() >= UDT_AMOUNT_LEN {
                        if self.is_sudt_cell(&cell.cell_output, &mut sudt_script_map) {
                            self.change_udt_balance(
                                &cell.cell_output,
                                &cell.cell_data.unpack(),
                                &mut sudt_balance_change,
                                true,
                            );
                        } else if self.is_xudt_cell(&cell.cell_output, &mut xudt_script_map) {
                            self.change_udt_balance(
                                &cell.cell_output,
                                &cell.cell_data.unpack(),
                                &mut xudt_balance_change,
                                true,
                            );
                        }
                    }
                }
            }

            for (output, data) in tx
                .outputs()
                .clone()
                .into_iter()
                .zip(tx.outputs_data().into_iter())
            {
                let data = data.raw_data();
                if data.len() >= UDT_AMOUNT_LEN {
                    if self.is_sudt_cell(&output, &mut sudt_script_map) {
                        self.change_udt_balance(&output, &data, &mut sudt_balance_change, false);
                    } else if self.is_xudt_cell(&output, &mut xudt_script_map) {
                        self.change_udt_balance(&output, &data, &mut xudt_balance_change, false);
                    }
                }
            }
        }

        self.store_balance(
            block.number(),
            &block.hash(),
            sudt_balance_map,
            sudt_script_map,
            xudt_balance_map,
            xudt_script_map,
        )?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let raw_data = self
            .store
            .get(block_key)?
            .expect("SUDT extension rollback data does not exist");

        let maps = UDTBalanceMaps::decode(&Rlp::new(&raw_data))?;
        let sudt_map = maps.sudt.clone().opposite_value();
        let xudt_map = maps.xudt.opposite_value();

        self.store_balance(
            tip_number,
            tip_hash,
            sudt_map,
            HashMap::default(),
            xudt_map,
            HashMap::default(),
        )?;

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

impl<S: Store, BS: Store> UDTBalanceExtension<S, BS> {
    pub fn new(
        store: S,
        indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        UDTBalanceExtension {
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
                format_err!(
                    "SUDT extension can not get live cell by out point, tx_hash {:?}, index {:?}",
                    out_point.tx_hash(),
                    out_point.index(),
                )
            })
    }

    // This function should be run after fn is_sudt_cell(&cell).
    fn extract_udt_address_key(&self, cell: &packed::CellOutput) -> Vec<u8> {
        let udt_hash: H256 = self.get_type_hash(cell).unwrap().unpack();
        let addr: [u8; 32] = cell.lock().calc_script_hash().unpack();
        let mut key = udt_hash.as_bytes().to_vec();
        key.extend_from_slice(&addr);
        key
    }

    fn change_udt_balance(
        &self,
        cell: &packed::CellOutput,
        cell_data: &Bytes,
        udt_balance_map: &mut HashMap<Vec<u8>, BigInt>,
        is_sub: bool,
    ) {
        // The key is include the udt type hash and the lock script hash.
        let key = self.extract_udt_address_key(cell);
        let udt_amount =
            u128::from_le_bytes(to_fixed_array::<UDT_AMOUNT_LEN>(&cell_data.to_vec()[0..16]));

        if is_sub {
            *udt_balance_map.entry(key).or_insert_with(BigInt::zero) -= udt_amount;
        } else {
            *udt_balance_map.entry(key).or_insert_with(BigInt::zero) += udt_amount;
        }
    }

    fn store_balance(
        &self,
        block_number: BlockNumber,
        block_hash: &packed::Byte32,
        sudt_balance_map: UDTBalanceMap,
        sudt_script_map: HashMap<packed::Byte32, packed::Script>,
        xudt_balance_map: UDTBalanceMap,
        xudt_script_map: HashMap<packed::Byte32, packed::Script>,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (addr, val) in sudt_balance_map.inner().iter() {
            self.store_udt_balance(addr, val, &mut batch)?;
        }

        for (addr, val) in xudt_balance_map.inner().iter() {
            self.store_udt_balance(addr, val, &mut batch)?;
        }

        for (script_hash, script) in sudt_script_map.iter() {
            batch.put_kv(Key::ScriptHash(script_hash), Value::Script(true, script))?;
        }

        for (script_hash, script) in xudt_script_map.iter() {
            batch.put_kv(Key::ScriptHash(script_hash), Value::Script(false, script))?;
        }

        batch.put_kv(
            Key::Block(block_number, block_hash),
            Value::RollbackData(
                UDTBalanceMaps::new(sudt_balance_map, xudt_balance_map).rlp_bytes(),
            ),
        )?;

        batch.commit()?;

        Ok(())
    }

    fn store_udt_balance(
        &self,
        addr: &[u8],
        val: &BigInt,
        batch: &mut <S as Store>::Batch,
    ) -> Result<()> {
        let key = Key::Address(&addr).into_vec();
        let original_balance = self.store.get(&key)?;
        if original_balance.is_none() && val < &Zero::zero() {
            return Err(UDTBalanceExtensionError::BalanceIsNegative {
                sudt_type_hash: hex::encode(&addr[0..32]),
                user_address: String::from_utf8(addr[32..].to_vec()).unwrap(),
                balance: val.clone(),
            }
            .into());
        }

        let current_balance = original_balance
            .map(|balance| u128::from_be_bytes(to_fixed_array(&balance)) + val)
            .unwrap_or_else(|| val.clone());

        if current_balance < Zero::zero() {
            return Err(UDTBalanceExtensionError::BalanceIsNegative {
                sudt_type_hash: hex::encode(&addr[0..32]),
                user_address: String::from_utf8(addr[32..].to_vec()).unwrap(),
                balance: current_balance,
            }
            .into());
        } else {
            let value: u128 = current_balance.try_into().unwrap();
            batch.put_kv(key, Value::SUDTBalance(value))?;
        }

        Ok(())
    }

    fn is_sudt_cell(
        &self,
        cell_output: &packed::CellOutput,
        sudt_cell_map: &mut HashMap<packed::Byte32, packed::Script>,
    ) -> bool {
        self.judge_udt(cell_output, UDTType::Simple, sudt_cell_map)
    }

    fn is_xudt_cell(
        &self,
        cell_output: &packed::CellOutput,
        xudt_cell_map: &mut HashMap<packed::Byte32, packed::Script>,
    ) -> bool {
        self.judge_udt(cell_output, UDTType::Extensible, xudt_cell_map)
    }

    fn judge_udt(
        &self,
        cell_output: &packed::CellOutput,
        udt_type: UDTType,
        udt_cell_map: &mut HashMap<packed::Byte32, packed::Script>,
    ) -> bool {
        cell_output
            .type_()
            .to_opt()
            .map(|script| {
                let sudt_config = self
                    .config
                    .get(udt_type.as_str())
                    .unwrap_or_else(|| panic!("{:?} extension config is empty", udt_type));

                if script.code_hash() == sudt_config.script.code_hash()
                    && script.hash_type() == sudt_config.script.hash_type()
                {
                    udt_cell_map.insert(script.calc_script_hash(), script);
                    return true;
                }

                false
            })
            .unwrap_or(false)
    }

    fn get_type_hash(&self, cell_output: &packed::CellOutput) -> Option<packed::Byte32> {
        cell_output.type_().to_opt().map(|s| s.calc_script_hash())
    }
}
