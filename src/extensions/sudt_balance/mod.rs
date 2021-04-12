use crate::extensions::{to_fixed_array, Array, Extension};
use crate::types::DeployedScriptConfig;

use anyhow::{format_err, Error, Result};
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::Store;
use ckb_types::core::BlockView;
use ckb_types::{bytes, packed, prelude::*, H256};
use num_bigint::BigInt;
use num_traits::identities::Zero;

use std::collections::HashMap;

#[repr(u8)]
pub enum KeyPrefix {
    SUDTBalacne = 253,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    SUDTAddress(&'a packed::Bytes, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::SUDTAddress(script_args, key) => {
                encoded.push(KeyPrefix::SUDTBalacne as u8);
                encoded.extend_from_slice(script_args.as_slice());
                encoded.extend_from_slice(key.as_slice());
            }
        }

        encoded
    }
}

pub enum Value {
    SUDTBalacne(u128),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::SUDTBalacne(balance) => Vec::from(balance.to_be_bytes()),
        }
    }
}

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
        let mut sudt_balance_change = HashMap::new();

        for tx in block.transactions().iter() {
            for input in tx.inputs().into_iter() {
                let cell = self.get_live_cell_by_out_point(&input.previous_output())?;

                if self.is_sudt_cell(&cell.cell_output) {
                    self.change_sudt_balance(
                        &cell.cell_output,
                        &cell.cell_data.unpack(),
                        &mut sudt_balance_change,
                        true,
                    );
                }
            }

            for (idx, output) in tx.outputs().into_iter().enumerate() {
                if self.is_sudt_cell(&output) {
                    self.change_sudt_balance(
                        &output,
                        &tx.outputs_data().get(idx).unwrap().unpack(),
                        &mut sudt_balance_change,
                        true,
                    );
                }
            }
        }
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        Ok(())
    }
}

impl<S: Clone + Store> BalanceExtension<S> {
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

    fn change_sudt_balance(
        &self,
        cell_output: &packed::CellOutput,
        cell_data: &bytes::Bytes,
        sudt_balance_map: &mut HashMap<Vec<u8>, BigInt>,
        is_sub: bool,
    ) {
        let sudt_id: H256 = self.get_type_hash(cell_output).unwrap().unpack();
        let addr = self.get_cell_lock_args(&cell_output);
        let mut key = sudt_id.as_bytes().to_vec();
        key.extend_from_slice(&addr.to_vec());

        let raw_sudt_amount = Array::<16>::from_slice(&cell_data.to_vec()[0..16]);
        let sudt_amount = u128::from_le_bytes(raw_sudt_amount.inner());

        if is_sub {
            *sudt_balance_map
                .entry(key)
                .or_insert_with(|| BigInt::zero()) -= sudt_amount;
        } else {
            *sudt_balance_map
                .entry(key)
                .or_insert_with(|| BigInt::zero()) += sudt_amount;
        }
    }

    fn is_sudt_cell(&self, cell_output: &packed::CellOutput) -> bool {
        cell_output
            .type_()
            .to_opt()
            .map(|script| {
                script.code_hash() == self.config.script.code_hash()
                    && script.hash_type() == self.config.script.hash_type()
            })
            .unwrap_or(false)
    }

    fn get_type_hash(&self, cell_output: &packed::CellOutput) -> Option<packed::Byte32> {
        cell_output
            .type_()
            .to_opt()
            .and_then(|s| Some(s.calc_script_hash()))
    }
}
