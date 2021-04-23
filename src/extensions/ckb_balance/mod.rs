mod types;

use types::{CkbBalanceExtensionError, CkbBalanceMap, Key, KeyPrefix, Value};

use crate::extensions::Extension;
use crate::types::DeployedScriptConfig;
use crate::utils::to_fixed_array;

use anyhow::Result;
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed, prelude::Unpack};
use rlp::{Decodable, Encodable, Rlp};

use std::collections::HashMap;
use std::sync::Arc;

pub struct CkbBalanceExtension<S, BS> {
    store: S,
    indexer: Arc<Indexer<BS>>,
    _config: DeployedScriptConfig,
}

impl<S: Store, BS: Store> Extension for CkbBalanceExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut ckb_balance_map = CkbBalanceMap::default();
        let mut ckb_balance_change = ckb_balance_map.inner_mut();

        if block.is_genesis() {
            return self.handle_genesis(block);
        }

        for tx in block.transactions().iter() {
            for input in tx.inputs().into_iter() {
                let cell = self.get_live_cell_by_out_point(&input.previous_output())?;
                self.change_ckb_balance(&cell.cell_output, &mut ckb_balance_change, true);
            }

            for output in tx.outputs().into_iter() {
                self.change_ckb_balance(&output, &mut ckb_balance_change, false);
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
            .expect("rollback data does not exist");

        let mut delta_map = CkbBalanceMap::decode(&Rlp::new(&map))?;
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
    pub fn new(store: S, indexer: Arc<Indexer<BS>>, _config: DeployedScriptConfig) -> Self {
        CkbBalanceExtension {
            store,
            indexer,
            _config,
        }
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or_else(|| CkbBalanceExtensionError::NoLiveCellByOutpoint(out_point.clone()).into())
    }

    fn get_cell_lock_args(&self, out_point: &packed::CellOutput) -> Bytes {
        out_point.lock().args().unpack()
    }

    fn get_cell_capacity(&self, cell_output: &packed::CellOutput) -> u64 {
        cell_output.capacity().unpack()
    }

    fn change_ckb_balance(
        &self,
        cell_output: &packed::CellOutput,
        ckb_balance_map: &mut HashMap<Bytes, i128>,
        is_sub: bool,
    ) {
        let addr = self.get_cell_lock_args(&cell_output);
        let capacity = self.get_cell_capacity(&cell_output);

        if is_sub {
            *ckb_balance_map.entry(addr).or_insert(0) -= capacity as i128;
        } else {
            *ckb_balance_map.entry(addr).or_insert(0) += capacity as i128;
        }
    }

    fn handle_genesis(&self, block: &BlockView) -> Result<()> {
        // The inputs in genesis block is empty. And it will not rollback.
        let mut batch = self.get_batch()?;

        for tx in block.transactions().iter() {
            for cell in tx.outputs().into_iter() {
                let addr = self.get_cell_lock_args(&cell);
                let balance: u64 = cell.capacity().unpack();
                batch.put_kv(Key::CkbAddress(&addr), Value::CkbBalance(balance))?;
            }
        }

        batch.commit()?;

        Ok(())
    }

    fn store_balance(
        &self,
        block_num: BlockNumber,
        block_hash: &packed::Byte32,
        ckb_balance_map: CkbBalanceMap,
    ) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (addr, val) in ckb_balance_map.inner().iter() {
            let key = Key::CkbAddress(&addr);
            let original_balance = self.store.get(addr)?;

            if original_balance.is_none() && *val < 0 {
                return Err(
                    CkbBalanceExtensionError::BalanceIsNegative(hex::encode(addr), *val).into(),
                );
            }

            let current_balance = original_balance
                .map(|balance| add(u64::from_be_bytes(to_fixed_array(&balance)), *val))
                .unwrap_or(*val);

            if current_balance < 0 {
                return Err(
                    CkbBalanceExtensionError::BalanceIsNegative(hex::encode(addr), *val).into(),
                );
            } else {
                batch.put_kv(key, Value::CkbBalance(current_balance as u64))?;
            }
        }

        batch.put_kv(
            Key::Block(block_num, &block_hash),
            Value::RollbackData(ckb_balance_map.rlp_bytes()),
        )?;

        batch.commit()?;

        Ok(())
    }

    #[cfg(test)]
    pub fn get_balance(&self, addr: &Bytes) -> Result<Option<u64>> {
        let bytes: Vec<u8> = Key::CkbAddress(addr).into();
        self.store
            .get(bytes)
            .map(|tmp| tmp.map(|bytes| u64::from_be_bytes(to_fixed_array(&bytes))))
            .map_err(Into::into)
    }
}

fn add(a: u64, b: i128) -> i128 {
    (a as i128) + b
}
