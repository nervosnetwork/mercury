use crate::extensions::{
    ckb_balance, special_cells, udt_balance, DetailedCells, CKB_EXT_PREFIX, SP_CELL_EXT_PREFIX,
    UDT_EXT_PREFIX,
};
use crate::rpc::rpc_impl::{address_to_script, MercuryRpcImpl};
use crate::{error::MercuryError, stores::add_prefix, utils::to_fixed_array};

use anyhow::Result;
use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_indexer::store::{IteratorDirection, Store};
use ckb_sdk::Address;
use ckb_types::{core::BlockNumber, packed, prelude::*, H160, H256};

use std::{convert::TryInto, iter::Iterator};

impl<S: Store> MercuryRpcImpl<S> {
    pub(crate) fn ckb_balance(&self, addr: &Address) -> Result<Option<u64>> {
        let addr = lock_hash(addr);
        let key = ckb_balance::Key::CkbAddress(&addr);
        let raw = self.store_get(*CKB_EXT_PREFIX, key.into_vec())?;

        Ok(raw.map(|bytes| u64::from_be_bytes(to_fixed_array(&bytes))))
    }

    pub(crate) fn udt_balance(&self, addr: &Address, udt_hash: H256) -> Result<Option<u128>> {
        let mut encoded = udt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&lock_hash(addr));
        let key = udt_balance::Key::Address(&encoded);

        let raw = self.store_get(*UDT_EXT_PREFIX, key.into_vec())?;
        Ok(raw.map(|bytes| u128::from_be_bytes(to_fixed_array(&bytes))))
    }

    pub(crate) fn get_sp_cells_by_addr(&self, addr: &Address) -> Result<DetailedCells> {
        let args = H160::from_slice(&addr.payload().args()).unwrap();
        let key = special_cells::Key::CkbAddress(&args);
        let ret = self.store_get(*SP_CELL_EXT_PREFIX, key.into_vec())?;

        if let Some(bytes) = ret {
            Ok(deserialize::<DetailedCells>(&bytes).unwrap())
        } else {
            Ok(Default::default())
        }
    }

    pub(crate) fn get_cells_by_lock_script(
        &self,
        lock_script: &packed::Script,
    ) -> Result<Vec<(DetailedLiveCell, packed::OutPoint)>> {
        let mut ret = Vec::new();
        let out_points =
            self.get_cells_by_script(lock_script, indexer::KeyPrefix::CellLockScript)?;

        for out_point in out_points.iter() {
            let cell = self.get_detailed_live_cell(out_point)?.ok_or_else(|| {
                MercuryError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                }
            })?;

            ret.push((cell, out_point.clone()));
        }

        Ok(ret)
    }

    fn get_cells_by_script(
        &self,
        script: &packed::Script,
        prefix: indexer::KeyPrefix,
    ) -> Result<Vec<packed::OutPoint>> {
        let mut start_key = vec![prefix as u8];
        start_key.extend_from_slice(&extract_raw_data(script));
        let iter = self.store.iter(&start_key, IteratorDirection::Forward)?;

        Ok(iter
            .take_while(|(key, _)| key.starts_with(&start_key))
            .map(|(key, value)| {
                let tx_hash = packed::Byte32::from_slice(&value).expect("stored tx hash");
                let index = OutputIndex::from_be_bytes(
                    key[key.len() - 4..].try_into().expect("stored index"),
                );
                packed::OutPoint::new(tx_hash, index)
            })
            .collect())
    }

    fn get_detailed_live_cell(
        &self,
        out_point: &packed::OutPoint,
    ) -> Result<Option<DetailedLiveCell>> {
        let key_vec = indexer::Key::OutPoint(&out_point).into_vec();
        let (block_number, tx_index, cell_output, cell_data) = match self.store.get(&key_vec)? {
            Some(stored_cell) => indexer::Value::parse_cell_value(&stored_cell),
            None => return Ok(None),
        };
        let mut header_start_key = vec![indexer::KeyPrefix::Header as u8];
        header_start_key.extend_from_slice(&block_number.to_be_bytes());
        let mut iter = self
            .store
            .iter(&header_start_key, IteratorDirection::Forward)?;
        let block_hash = match iter.next() {
            Some((key, _)) => {
                if key.starts_with(&header_start_key) {
                    let start = std::mem::size_of::<BlockNumber>() + 1;
                    packed::Byte32::from_slice(&key[start..start + 32])
                        .expect("stored key header hash")
                } else {
                    return Ok(None);
                }
            }
            None => return Ok(None),
        };

        Ok(Some(DetailedLiveCell {
            block_number,
            block_hash,
            tx_index,
            cell_output,
            cell_data,
        }))
    }

    pub(crate) fn store_get<P: AsRef<[u8]>, K: AsRef<[u8]>>(
        &self,
        prefix: P,
        key: K,
    ) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(prefix, key)).map_err(Into::into)
    }
}

fn lock_hash(addr: &Address) -> [u8; 32] {
    let script = address_to_script(addr.payload());
    script.calc_script_hash().unpack()
}
