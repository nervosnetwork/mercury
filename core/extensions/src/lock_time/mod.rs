pub mod types;

use types::{CellbaseCkb, CellbaseCkbAccount, CellbaseWithAddress, Key, KeyPrefix, Value};

use crate::{types::DeployedScriptConfig, Extension};

use common::anyhow::Result;
use common::utils::to_fixed_array;

use bincode::{deserialize, serialize};
use ckb_indexer::indexer::Indexer;
use ckb_indexer::store::{Batch, IteratorDirection, Store};
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*};

use std::collections::HashMap;
use std::sync::Arc;

pub struct LocktimeExtension<S, BS> {
    store: S,
    _indexer: Arc<Indexer<BS>>,
    _net_ty: NetworkType,
    _config: HashMap<String, DeployedScriptConfig>,
}

impl<S: Store, BS: Store> Extension for LocktimeExtension<S, BS> {
    fn append(&self, block: &BlockView) -> Result<()> {
        if block.is_genesis() {
            return Ok(());
        }

        let mut batch = self.get_batch()?;
        let mut addr = [0u8; 32];

        if let Some(cellbase) = block.transaction(0).unwrap().output(0) {
            addr = cellbase.lock().calc_script_hash().unpack();

            self.update_data(&addr, block, &cellbase, &mut batch)?;
        }

        self.mature_others(&addr, &mut batch)?;
        batch.commit()?;

        Ok(())
    }

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()> {
        let block_key = Key::Block(tip_number, tip_hash).into_vec();
        let raw_data = self
            .store
            .get(&block_key)?
            .expect("Lock time extension rollback data is not exist");

        let cellbase_with_address = deserialize::<CellbaseWithAddress>(&raw_data).unwrap();
        let mut account = self.get_cellbase_account(&cellbase_with_address.address)?;
        account.remove(&cellbase_with_address.cellbase);

        let mut batch = self.get_batch()?;
        batch.put_kv(
            Key::CkbAddress(&cellbase_with_address.address).into_vec(),
            Value::CellbaseCapacity(serialize(&account).unwrap()),
        )?;
        batch.commit()?;

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

impl<S: Store, BS: Store> LocktimeExtension<S, BS> {
    pub fn new(
        store: S,
        _indexer: Arc<Indexer<BS>>,
        _net_ty: NetworkType,
        _config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        LocktimeExtension {
            store,
            _indexer,
            _net_ty,
            _config,
        }
    }

    fn get_cellbase_account(&self, addr: &[u8; 32]) -> Result<CellbaseCkbAccount> {
        if let Some(bytes) = self.store.get(Key::CkbAddress(addr).into_vec())? {
            let account = deserialize::<CellbaseCkbAccount>(&bytes).unwrap();
            Ok(account)
        } else {
            Ok(Default::default())
        }
    }

    fn update_data(
        &self,
        addr: &[u8; 32],
        block: &BlockView,
        cellbase: &packed::CellOutput,
        batch: &mut S::Batch,
    ) -> Result<()> {
        let cellbase_ckb =
            CellbaseCkb::new(block.epoch().to_rational(), cellbase.capacity().unpack());
        let mut account = self.get_cellbase_account(addr)?;
        account.push(cellbase_ckb.clone());
        account.mature();
        let cellbase_with_address = CellbaseWithAddress::new(*addr, cellbase_ckb);

        batch.put_kv(
            Key::CkbAddress(addr).into_vec(),
            Value::CellbaseCapacity(serialize(&account).unwrap()),
        )?;
        batch.put_kv(
            Key::Block(block.number(), &block.hash()),
            Value::RollbackData(serialize(&cellbase_with_address).unwrap()),
        )?;

        Ok(())
    }

    fn mature_others(&self, except: &[u8; 32], batch: &mut S::Batch) -> Result<()> {
        let except_key = Key::CkbAddress(except).into_vec();
        let start_key = vec![KeyPrefix::Address as u8];
        let iter = self.store.iter(&start_key, IteratorDirection::Forward)?;
        let mut new_data = Vec::new();

        for (key, val) in iter.skip_while(|(key, _)| key.as_ref() == except_key) {
            let mut account = deserialize::<CellbaseCkbAccount>(&val).unwrap();
            account.mature();
            new_data.push((key, Value::CellbaseCapacity(serialize(&account).unwrap())));
        }

        for (key, val) in new_data.into_iter() {
            batch.put_kv(key.to_vec().split_off(10), val)?;
        }

        Ok(())
    }

    fn get_batch(&self) -> Result<S::Batch> {
        self.store.batch().map_err(Into::into)
    }
}
