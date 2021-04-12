mod types;

use types::{CkbBalanceExtensionError, Key, KeyPrefix, Value};

use crate::extensions::{to_fixed_array, Extension};
use crate::types::DeployedScriptConfig;

use anyhow::{format_err, Error, Result};
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::{Batch, Store};
use ckb_types::core::BlockView;
use ckb_types::{bytes, packed, prelude::*};

use std::collections::HashMap;

pub struct BalanceExtension<S> {
    store:   S,
    indexer: Indexer<S>,
    config:  DeployedScriptConfig,
}

impl<S> Extension for BalanceExtension<S>
where
    S: Clone + Store + 'static,
{
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut ckb_balance_change = HashMap::new();

        for tx in block.transactions().iter() {
            for input in tx.inputs().into_iter() {
                let cell = self.get_live_cell_by_out_point(&input.previous_output())?;
                self.change_ckb_balance(&cell.cell_output, &mut ckb_balance_change, true);
            }

            for output in tx.outputs().into_iter() {
                self.change_ckb_balance(&output, &mut ckb_balance_change, false);
            }
        }

        self.store_balance(ckb_balance_change)?;

        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        Ok(())
    }
}

impl<S> BalanceExtension<S>
where
    S: Clone + Store + 'static,
{
    pub fn new(store: S, indexer: Indexer<S>, config: DeployedScriptConfig) -> Self {
        BalanceExtension {
            store,
            indexer,
            config,
        }
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch()
    }

    fn get_live_cell_by_out_point(&self, out_point: &packed::OutPoint) -> Result<DetailedLiveCell> {
        self.indexer
            .get_detailed_live_cell(out_point)?
            .ok_or(format_err!(
                "cannot get live cell by out point {:?}",
                out_point
            ))
    }

    fn get_cell_lock_args(&self, out_point: &packed::CellOutput) -> bytes::Bytes {
        out_point.lock().args().unpack()
    }

    fn get_cell_capacity(&self, cell_output: &packed::CellOutput) -> u64 {
        cell_output.capacity().unpack()
    }

    fn change_ckb_balance(
        &self,
        cell_output: &packed::CellOutput,
        ckb_balance_map: &mut HashMap<bytes::Bytes, i128>,
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

    fn store_balance(&self, ckb_balance_map: HashMap<bytes::Bytes, i128>) -> Result<()> {
        let mut batch = self.get_batch()?;

        for (addr, val) in ckb_balance_map.into_iter() {
            let key = Key::CkbAddress(&addr);
            let original_balance = self.store.get(&addr)?;
            if original_balance.is_none() && val < 0 {
                return Err(
                    CkbBalanceExtensionError::BalanceIsNegative(hex::encode(addr), val).into(),
                );
            }

            let current_balance = original_balance
                .map(|balance| add(u64::from_le_bytes(to_fixed_array(&balance)), val))
                .unwrap_or(val);

            if current_balance < 0 {
                return Err(
                    CkbBalanceExtensionError::BalanceIsNegative(hex::encode(addr), val).into(),
                );
            } else {
                batch.put_kv(key, Value::CkbBalance(current_balance as u64))?;
            }
        }

        batch.commit()?;
        
        Ok(())
    }
}

fn add(a: u64, b: i128) -> i128 {
    (a as i128) + b
}
