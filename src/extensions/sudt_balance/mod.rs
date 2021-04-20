mod types;

use rlp::Encodable;
use types::{Key, KeyPrefix, SUDTBalanceExtensionError, SUDTBalanceMap, Value};

use crate::extensions::{to_fixed_array, Extension};
use crate::types::DeployedScriptConfig;

use anyhow::{format_err, Result};
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::BlockView;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Unpack, H256};
use num_bigint::BigInt;
use num_traits::identities::Zero;
use rlp::{Decodable, Rlp};

use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;

const SUDT_AMOUNT_LEN: usize = 16;

#[allow(clippy::upper_case_acronyms)]
pub struct SUDTBalanceExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    config: DeployedScriptConfig,
}

impl<S: Store, BS: Store> Extension for SUDTBalanceExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut sudt_balance_map = SUDTBalanceMap::default();
        let mut sudt_balance_change = sudt_balance_map.inner_mut();
        let mut sudt_script_map = HashMap::new();

        for tx in block.transactions().iter() {
            for input in tx.inputs().into_iter() {
                let cell = self.get_live_cell_by_out_point(&input.previous_output())?;

                if self.is_sudt_cell(&cell.cell_output, &mut sudt_script_map) {
                    self.change_sudt_balance(
                        &cell.cell_output,
                        &cell.cell_data.unpack(),
                        &mut sudt_balance_change,
                        true,
                    );
                }
            }

            for (idx, output) in tx.outputs().into_iter().enumerate() {
                if self.is_sudt_cell(&output, &mut sudt_script_map) {
                    self.change_sudt_balance(
                        &output,
                        &tx.outputs_data().get(idx).unwrap().unpack(),
                        &mut sudt_balance_change,
                        true,
                    );
                }
            }
        }

        self.store_balance(
            block.number(),
            &block.hash(),
            sudt_balance_map,
            sudt_script_map,
        )?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let map = self
            .store
            .get(block_key)?
            .expect("rollback data does not exist");

        let delta_map = SUDTBalanceMap::decode(&Rlp::new(&map))?;
        let map = delta_map.opposite_value();

        self.store_balance(tip_number, tip_hash, map, HashMap::default())?;

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

impl<S: Store, BS: Store> SUDTBalanceExtension<S, BS> {
    pub fn new(store: S, indexer: Arc<Indexer<BS>>, config: DeployedScriptConfig) -> Self {
        SUDTBalanceExtension {
            store,
            indexer,
            config,
        }
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into).into()
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or_else(|| format_err!("cannot get live cell by out point {:?}", out_point))
    }

    fn get_cell_lock_args(&self, out_point: &packed::CellOutput) -> Bytes {
        out_point.lock().args().unpack()
    }

    fn change_sudt_balance(
        &self,
        cell_output: &packed::CellOutput,
        cell_data: &Bytes,
        sudt_balance_map: &mut HashMap<Vec<u8>, BigInt>,
        is_sub: bool,
    ) {
        // This function runs when cell.is_sudt_cell() == true, so this unwrap() is safe.
        let sudt_id: H256 = self.get_type_hash(cell_output).unwrap().unpack();
        let addr = self.get_cell_lock_args(&cell_output);
        let mut key = sudt_id.as_bytes().to_vec();
        key.extend_from_slice(&addr.to_vec());

        let sudt_amount = u128::from_le_bytes(to_fixed_array::<SUDT_AMOUNT_LEN>(
            &cell_data.to_vec()[0..16],
        ));

        if is_sub {
            *sudt_balance_map.entry(key).or_insert_with(BigInt::zero) -= sudt_amount;
        } else {
            *sudt_balance_map.entry(key).or_insert_with(BigInt::zero) += sudt_amount;
        }
    }

    fn store_balance(
        &self,
        block_number: BlockNumber,
        block_hash: &packed::Byte32,
        sudt_balance_map: SUDTBalanceMap,
        sudt_script_map: HashMap<packed::Byte32, packed::Script>,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (addr, val) in sudt_balance_map.inner().iter() {
            let key = Key::Address(&addr);
            let original_balance = self.store.get(addr)?;

            if original_balance.is_none() && val < &Zero::zero() {
                return Err(SUDTBalanceExtensionError::BalanceIsNegative {
                    sudt_type_hash: hex::encode(&addr[0..32]),
                    user_address: hex::encode(&addr[32..]),
                    balance: val.clone(),
                }
                .into());
            }

            let current_balance = original_balance
                .map(|balance| u128::from_be_bytes(to_fixed_array(&balance)) + val)
                .unwrap_or_else(|| val.clone());

            if current_balance < Zero::zero() {
                return Err(SUDTBalanceExtensionError::BalanceIsNegative {
                    sudt_type_hash: hex::encode(&addr[0..32]),
                    user_address: hex::encode(&addr[32..]),
                    balance: current_balance,
                }
                .into());
            } else {
                let value: u128 = current_balance.try_into().unwrap();
                batch.put_kv(key, Value::SUDTBalacne(value))?;
            }
        }

        for (script_hash, script) in sudt_script_map.iter() {
            batch.put_kv(Key::ScriptHash(script_hash), Value::Script(script))?;
        }

        batch.put_kv(
            Key::Block(block_number, block_hash),
            Value::RollbackData(sudt_balance_map.rlp_bytes()),
        )?;

        batch.commit()?;

        Ok(())
    }

    fn is_sudt_cell(
        &self,
        cell_output: &packed::CellOutput,
        sudt_cell_map: &mut HashMap<packed::Byte32, packed::Script>,
    ) -> bool {
        cell_output
            .type_()
            .to_opt()
            .map(|script| {
                if script.code_hash() == self.config.script.code_hash()
                    && script.hash_type() == self.config.script.hash_type()
                {
                    sudt_cell_map.insert(script.calc_script_hash(), script);
                }
                true
            })
            .unwrap_or(false)
    }

    fn get_type_hash(&self, cell_output: &packed::CellOutput) -> Option<packed::Byte32> {
        cell_output.type_().to_opt().map(|s| s.calc_script_hash())
    }
}
