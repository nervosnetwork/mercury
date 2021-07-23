use crate::rpc_impl::{
    address_to_script, parse_key_address, parse_normal_address, pubkey_to_secp_address,
    MercuryRpcImpl, CURRENT_BLOCK_NUMBER,
};
use crate::types::{
    Balance, GenericTransaction, GetBalanceResponse, InnerBalance, OrderEnum, QueryAddress,
    ScriptType, TxScriptLocation,
};
use crate::{block_on, error::RpcError, CkbRpc};

use common::utils::{decode_udt_amount, parse_address, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError};
use core_extensions::{
    ckb_balance, lock_time, script_hash, special_cells, udt_balance, DetailedCells, CKB_EXT_PREFIX,
    CURRENT_EPOCH, LOCK_TIME_PREFIX, SCRIPT_HASH_EXT_PREFIX, SP_CELL_EXT_PREFIX, UDT_EXT_PREFIX,
};
use core_storage::{add_prefix, IteratorDirection, Store};

use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{packed, prelude::*, H160, H256};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

use lazysort::SortedBy;

impl<S, C> MercuryRpcImpl<S, C>
where
    S: Store,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    pub(crate) fn inner_get_balance(
        &self,
        udt_hashes: HashSet<Option<H256>>,
        address: QueryAddress,
        block_number: Option<u64>,
    ) -> Result<GetBalanceResponse> {
        // todo: After support search according to height, the process should be:  if search for latest height (default mode), we should get `CURRENT_BLOCK_NUMBER` first, and then query according to height
        let block_num = if block_number.is_some() {
            return Err(MercuryError::rpc(RpcError::GetBalanceByBlockNumberNotSupportYet).into());
        } else {
            **CURRENT_BLOCK_NUMBER.load()
        };

        let udt_hashes = if udt_hashes.is_empty() {
            self.get_all_udt_hashes()?
        } else {
            udt_hashes
        };

        let bal = match address {
            QueryAddress::KeyAddress(addr) => {
                let addr = parse_key_address(&addr)?;
                let mut balances = Vec::new();
                let sp_cells = self.get_sp_detailed_cells(&addr)?;

                for hash in udt_hashes.into_iter() {
                    let unconstrained =
                        self.get_unconstrained_balance(hash.clone(), &addr, &sp_cells)?;
                    let locked = self.get_locked_balance(hash.clone(), &addr, &sp_cells)?;
                    let fleeting = self.get_fleeting_balance(hash.clone(), &addr, &sp_cells)?;
                    balances.push(Balance::new(
                        addr.to_string(),
                        hash,
                        unconstrained,
                        fleeting,
                        locked,
                    ));
                }
                balances
            }

            QueryAddress::NormalAddress(addr) => {
                let addr = parse_normal_address(&addr)?;
                let script = address_to_script(addr.payload());

                if self.is_secp256k1(&script) {
                    self.inner_get_secp_balance(udt_hashes, &addr)?
                } else if self.is_acp(&script) {
                    self.inner_get_acp_balance(udt_hashes, &addr)?
                } else {
                    return Err(MercuryError::rpc(RpcError::UnsupportedNormalAddress).into());
                }
            }
        };

        Ok(GetBalanceResponse::new(block_num, bal))
    }

    pub(crate) fn inner_get_secp_balance(
        &self,
        udt_hashes: HashSet<Option<H256>>,
        addr: &Address,
    ) -> Result<Vec<Balance>> {
        let mut balances = Vec::new();

        for hash in udt_hashes.into_iter() {
            let unconstrained = if let Some(hash) = hash.clone() {
                self.udt_balance(addr, &hash)?.unwrap_or(0)
            } else {
                self.ckb_balance(addr)? as u128
            };

            balances.push(Balance::new(addr.to_string(), hash, unconstrained, 0, 0));
        }

        Ok(balances)
    }

    pub(crate) fn inner_get_acp_balance(
        &self,
        udt_hashes: HashSet<Option<H256>>,
        addr: &Address,
    ) -> Result<Vec<Balance>> {
        let mut balances = udt_hashes
            .into_iter()
            .map(|hash| {
                (
                    hash.clone(),
                    InnerBalance::new(self.acp_addr_to_secp(addr).to_string(), hash),
                )
            })
            .collect::<HashMap<_, _>>();
        let script = address_to_script(addr.payload());
        let key = pubkey_to_secp_address(addr.payload().args());
        let cells = self
            .store_get(
                *SP_CELL_EXT_PREFIX,
                special_cells::Key::CkbAddress(&key).into_vec(),
            )?
            .map_or_else(DetailedCells::default, |bytes| deserialize(&bytes).unwrap());
        let mut ckb_locked = 0;

        for cell in cells
            .0
            .iter()
            .filter(|cell| cell.cell_output.lock() == script)
        {
            if cell.cell_output.type_().is_none() {
                if let Some(bal) = balances.get_mut(&None) {
                    let unconstrained: u64 = cell.cell_output.capacity().unpack();
                    bal.unconstrained += unconstrained;
                }
            } else {
                let udt_hash: H256 = cell
                    .cell_output
                    .type_()
                    .to_opt()
                    .unwrap()
                    .calc_script_hash()
                    .unpack();
                if let Some(bal) = balances.get_mut(&Some(udt_hash)) {
                    let locked: u64 = cell.cell_output.capacity().unpack();
                    let unconstrained = decode_udt_amount(&cell.cell_data.raw_data());
                    bal.unconstrained += unconstrained;
                    ckb_locked += locked;
                }
            }
        }

        if let Some(bal) = balances.get_mut(&None) {
            bal.locked += ckb_locked;
        }

        Ok(balances.into_iter().map(|(_k, v)| v.into()).collect())
    }

    pub(crate) fn get_unconstrained_balance(
        &self,
        udt_hash: Option<H256>,
        addr: &Address,
        sp_cells: &DetailedCells,
    ) -> Result<u128> {
        if let Some(hash) = udt_hash {
            let unconstrained_udt_balance = self.udt_balance(addr, &hash)?.unwrap_or(0);
            let acp_unconstrained_udt_balance =
                self.acp_unconstrained_udt_balance(hash.clone(), sp_cells)? as u128;
            let cheque_unconstrained_udt_balance =
                self.cheque_unconstrained_udt_balance(addr, hash)?;
            let total_unconstrained_udt_balance = unconstrained_udt_balance
                + acp_unconstrained_udt_balance
                + cheque_unconstrained_udt_balance;
            Ok(total_unconstrained_udt_balance)
        } else {
            let unconstrained_ckb_balance = self.ckb_balance(addr)? as u128;
            let acp_unconstrained_ckb_balance =
                self.acp_unconstrained_ckb_balance(sp_cells)? as u128;
            let cellbase_locked_ckb_balance = self.cellbase_locked_ckb_balance(addr)? as u128;
            let total_unconstrained_ckb_balance = unconstrained_ckb_balance
                + acp_unconstrained_ckb_balance
                - cellbase_locked_ckb_balance;
            Ok(total_unconstrained_ckb_balance)
        }
    }

    pub(crate) fn get_fleeting_balance(
        &self,
        udt_hash: Option<H256>,
        addr: &Address,
        sp_cells: &DetailedCells,
    ) -> Result<u128> {
        let fleeting_balance = if let Some(hash) = udt_hash {
            self.cheque_fleeting_udt_balance(addr, hash, sp_cells)?
        } else {
            0
        };
        Ok(fleeting_balance)
    }

    pub(crate) fn get_locked_balance(
        &self,
        udt_hash: Option<H256>,
        addr: &Address,
        sp_cells: &DetailedCells,
    ) -> Result<u128> {
        if udt_hash.is_some() {
            return Ok(0u128);
        }

        let cellbase_locked_balance = self.cellbase_locked_ckb_balance(addr)?;
        let acp_locked_balance = self.acp_locked_ckb_balance(sp_cells)?;
        let cheque_locked_balance = self.cheque_locked_ckb_balance(addr)?;
        let total_locked_balance =
            cellbase_locked_balance + acp_locked_balance + cheque_locked_balance;

        Ok(total_locked_balance as u128)
    }

    pub(crate) fn cellbase_locked_ckb_balance(&self, addr: &Address) -> Result<u64> {
        let lock_hash = lock_hash(addr);
        let key = lock_time::types::Key::CkbAddress(&lock_hash);
        let value = self.store_get(*LOCK_TIME_PREFIX, key.into_vec())?;
        let immature_cellbase_ckb = if let Some(raw) = value {
            let cellbase_ckb_account = deserialize::<lock_time::types::CellbaseCkbAccount>(&raw)?;
            cellbase_ckb_account
                .immature
                .iter()
                .map(|item| item.capacity.as_u64())
                .sum()
        } else {
            0
        };

        Ok(immature_cellbase_ckb)
    }

    pub(crate) fn acp_locked_ckb_balance(&self, sp_cells: &DetailedCells) -> Result<u64> {
        let config = self.get_config(special_cells::ACP)?;
        let locked_capacity: u64 = sp_cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
                    && cell.cell_output.type_().is_some()
            })
            .map(|cell| {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity
            })
            .sum();
        Ok(locked_capacity)
    }

    pub(crate) fn acp_unconstrained_ckb_balance(&self, sp_cells: &DetailedCells) -> Result<u64> {
        let config = self.get_config(special_cells::ACP)?;
        let unconstrained_ckb_balance = sp_cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
                    && cell.cell_output.type_().is_none()
            })
            .map(|cell| {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity
            })
            .sum();
        Ok(unconstrained_ckb_balance)
    }

    pub(crate) fn acp_unconstrained_udt_balance(
        &self,
        udt_hash: H256,
        sp_cells: &DetailedCells,
    ) -> Result<u128> {
        let config = self.get_config(special_cells::ACP)?;
        let unconstrained_udt_balance = sp_cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
                    && cell.cell_output.type_().is_some()
            })
            .filter(|cell| {
                let type_script = cell.cell_output.type_().to_opt().unwrap();
                let type_script_hash: [u8; 32] = type_script.calc_script_hash().unpack();
                type_script_hash == udt_hash.0
            })
            .map(|cell| decode_udt_amount(&cell.cell_data.raw_data()))
            .sum();
        Ok(unconstrained_udt_balance)
    }

    pub(crate) fn cheque_unconstrained_udt_balance(
        &self,
        addr: &Address,
        udt_hash: H256,
    ) -> Result<u128> {
        let script = address_to_script(addr.payload());
        let pubkey_hash = H160::from_slice(&script.args().raw_data()[0..20]).unwrap();
        let cells = self.get_sp_detailed_cells(addr)?;
        let config = self.get_config(special_cells::CHEQUE)?;
        let current_epoch = {
            let epoch = CURRENT_EPOCH.read();
            epoch.clone()
        };
        let unconstrained_udt_balance = cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
                    && cell.cell_output.type_().is_some()
            })
            .filter(|cell| {
                let type_script = cell.cell_output.type_().to_opt().unwrap();
                let type_script_hash: [u8; 32] = type_script.calc_script_hash().unpack();
                type_script_hash == udt_hash.0
            })
            .filter(|cell| {
                // filter sender pubkey_hash
                let lock_args = cell.cell_output.lock().args().raw_data();
                lock_args.len() == 40 && lock_args[20..40] == pubkey_hash.0
            })
            .filter(move |cell| {
                let cell_epoch = RationalU256::from_u256(cell.epoch_number.clone());
                let cheque_since = self.cheque_since.clone();
                current_epoch.clone().sub(cell_epoch) >= cheque_since
            })
            .map(|cell| decode_udt_amount(&cell.cell_data.raw_data()))
            .sum();
        Ok(unconstrained_udt_balance)
    }

    pub(crate) fn cheque_fleeting_udt_balance(
        &self,
        addr: &Address,
        udt_hash: H256,
        cells: &DetailedCells,
    ) -> Result<u128> {
        let script = address_to_script(addr.payload());
        let lock_hash = blake2b_160(script.as_slice());
        let config = self.get_config(special_cells::CHEQUE)?;
        let current_epoch = {
            let epoch = CURRENT_EPOCH.read();
            epoch.clone()
        };
        let fleeting_udt_balance = cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
                    && cell.cell_output.type_().is_some()
            })
            .filter(|cell| {
                let type_script = cell.cell_output.type_().to_opt().unwrap();
                let type_script_hash: [u8; 32] = type_script.calc_script_hash().unpack();
                type_script_hash == udt_hash.0
            })
            .filter(|cell| {
                // filter receiver pubkey_hash
                let lock_args = cell.cell_output.lock().args().raw_data();
                lock_args.len() == 40 && lock_args[0..20] == lock_hash
            })
            .filter(move |cell| {
                let cell_epoch = RationalU256::from_u256(cell.epoch_number.clone());
                let cheque_since = self.cheque_since.clone();
                current_epoch.clone().sub(cell_epoch) < cheque_since
            })
            .map(|cell| decode_udt_amount(&cell.cell_data.raw_data()))
            .sum();
        Ok(fleeting_udt_balance)
    }

    pub(crate) fn cheque_locked_ckb_balance(&self, addr: &Address) -> Result<u64> {
        let script = address_to_script(addr.payload());
        let pubkey_hash = H160::from_slice(&script.args().raw_data()[0..20]).unwrap();
        let cells = self.get_sp_detailed_cells(addr)?;
        let config = self.get_config(special_cells::CHEQUE)?;
        let locked_ckb_balance = cells
            .0
            .iter()
            .filter(|cell| {
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
            })
            .filter(|cell| {
                // filter sender pubkey_hash
                let lock_args = cell.cell_output.lock().args().raw_data();
                lock_args.len() == 40 && lock_args[20..40] == pubkey_hash.0
            })
            .map(|cell| {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity
            })
            .sum();
        Ok(locked_ckb_balance)
    }

    pub(crate) fn get_sp_detailed_cells(&self, addr: &Address) -> Result<DetailedCells> {
        let script_hash = H160(blake2b_160(address_to_script(addr.payload()).as_slice()));
        let key = special_cells::Key::CkbAddress(&script_hash);
        let cells = self
            .store_get(*SP_CELL_EXT_PREFIX, key.into_vec())?
            .map_or_else(DetailedCells::default, |bytes| deserialize(&bytes).unwrap());
        Ok(cells)
    }

    pub(crate) fn ckb_balance(&self, addr: &Address) -> Result<u64> {
        let addr = lock_hash(addr);
        let key = ckb_balance::Key::CkbAddress(&addr);
        let balance = self
            .store_get(*CKB_EXT_PREFIX, key.into_vec())?
            .map_or_else(ckb_balance::Balance::default, |bytes| {
                deserialize(&bytes).unwrap()
            });
        Ok(balance.normal_cell_capacity + balance.udt_cell_capacity)
    }

    pub(crate) fn udt_balance(&self, addr: &Address, udt_hash: &H256) -> Result<Option<u128>> {
        let mut encoded = udt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&lock_hash(addr));
        let key = udt_balance::Key::Address(&encoded);

        let raw = self.store_get(*UDT_EXT_PREFIX, key.into_vec())?;
        Ok(raw.map(|bytes| u128::from_be_bytes(to_fixed_array(&bytes))))
    }

    pub(crate) fn get_sp_cells_by_addr(&self, addr: &Address) -> Result<DetailedCells> {
        let hash = H160(blake2b_160(address_to_script(addr.payload()).as_slice()));
        let key = special_cells::Key::CkbAddress(&hash);
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
                MercuryError::rpc(RpcError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                })
            })?;

            ret.push((cell, out_point.clone()));
        }

        Ok(ret)
    }

    // The script_types argument is reserved for future.
    pub(crate) fn get_transactions_by_scripts(
        &self,
        address: &Address,
        _script_types: Vec<ScriptType>,
    ) -> Result<Vec<H256>> {
        let mut ret = HashSet::new();
        let mut lock_scripts = self
            .get_sp_cells_by_addr(&address)?
            .0
            .into_iter()
            .map(|cell| cell.cell_output.lock())
            .collect::<HashSet<_>>();
        lock_scripts.insert(address_to_script(address.payload()));

        for lock_script in lock_scripts.iter() {
            let hashes = self
                .get_transactions_by_script(lock_script, indexer::KeyPrefix::TxLockScript)?
                .into_iter()
                .map(|hash| hash.unpack())
                .collect::<Vec<H256>>();

            ret.extend(hashes);
        }

        Ok(ret.into_iter().collect())
    }

    pub(crate) fn inner_query_transactions(
        &self,
        address: QueryAddress,
        udt_hashes: HashSet<Option<H256>>,
        from_block: u64,
        to_block: u64,
        offset: u64,
        limit: u64,
        order: OrderEnum,
    ) -> Result<Vec<GenericTransaction>> {
        let mut ret = Vec::new();
        let udt_hashes = if udt_hashes.is_empty() {
            self.get_all_udt_hashes()?
        } else {
            udt_hashes
        };

        let tx_hashes = self.get_tx_hashes_by_query_options(
            address, udt_hashes, from_block, to_block, offset, limit, order,
        )?;
        let hash_clone = tx_hashes.clone();
        let txs_with_status = block_on!(self, get_transactions, hash_clone)?;

        for (index, item) in txs_with_status.into_iter().enumerate() {
            if let Some(tx) = item {
                let tx_hash = tx.transaction.hash;
                let tx_status = tx.tx_status.status;
                let generic_transaction = self.inner_get_generic_transaction(
                    tx.transaction.inner.into(),
                    tx_hash,
                    tx_status,
                    None,
                    None,
                    None,
                )?;
                ret.push(generic_transaction.into());
            } else {
                let tx_hash = tx_hashes.get(index).unwrap();
                return Err(
                    MercuryError::rpc(RpcError::CannotGetTxByHash(hex::encode(tx_hash))).into(),
                );
            }
        }
        Ok(ret)
    }

    pub(crate) fn get_tx_hashes_by_query_options(
        &self,
        address: QueryAddress,
        udt_hashes: HashSet<Option<H256>>,
        from_block: u64,
        to_block: u64,
        offset: u64,
        limit: u64,
        order: OrderEnum,
    ) -> Result<Vec<H256>> {
        let tx_hashes = match address {
            QueryAddress::KeyAddress(key_address) => {
                let address = parse_key_address(&key_address)?;
                let mut lock_scripts = self
                    .get_sp_cells_by_addr(&address)?
                    .0
                    .into_iter()
                    .map(|cell| cell.cell_output.lock())
                    .collect::<HashSet<_>>();
                lock_scripts.insert(address_to_script(address.payload()));
                let mut script_locations: Vec<TxScriptLocation> = vec![];
                for lock_script in lock_scripts {
                    let mut lock_script_locations = self.get_transaction_script_locations(
                        lock_script,
                        udt_hashes.clone(),
                        from_block,
                        to_block,
                    )?;
                    script_locations.append(&mut lock_script_locations);
                }
                order_then_paginate(script_locations, order, offset, limit)
            }
            QueryAddress::NormalAddress(normal_address) => {
                let address = parse_normal_address(&normal_address)?;
                let lock_script = address_to_script(&address.payload());
                let lock_script_locations = self.get_transaction_script_locations(
                    lock_script,
                    udt_hashes,
                    from_block,
                    to_block,
                )?;
                order_then_paginate(lock_script_locations, order, offset, limit)
            }
        };
        Ok(tx_hashes)
    }

    pub(crate) fn get_transaction_script_locations(
        &self,
        lock_script: packed::Script,
        udt_hashes: HashSet<Option<H256>>,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<TxScriptLocation>> {
        let lock_script_locations = self.get_transaction_script_locations_by_script(
            &lock_script,
            indexer::KeyPrefix::TxLockScript,
            from_block,
            to_block,
        )?;

        log::trace!("lock_script_locations: {:?}", lock_script_locations);

        let filtered_tx_script_locations = lock_script_locations
            .into_iter()
            .filter(|lock_script_location| {
                let tx_hash: [u8; 32] = lock_script_location.tx_hash.unpack();
                let key = script_hash::types::Key::CellTypeHash(
                    tx_hash,
                    lock_script_location.io_index,
                    lock_script_location.io_type,
                );
                let type_hash = self
                    .store_get(*SCRIPT_HASH_EXT_PREFIX, key.into_vec())
                    .unwrap()
                    .unwrap();
                packed::Byte32::from_slice(&type_hash).unwrap().is_zero()
                    && udt_hashes.contains(&None)
                    || udt_hashes.contains(&Some(H256::from_slice(&type_hash).unwrap()))
            })
            .collect();

        log::trace!(
            "filtered_tx_script_locations: {:?}",
            filtered_tx_script_locations
        );
        Ok(filtered_tx_script_locations)
    }

    pub(crate) fn get_transaction_script_locations_by_script(
        &self,
        script: &packed::Script,
        prefix: indexer::KeyPrefix,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<TxScriptLocation>> {
        // Key::TxLockScript/Key::TxTypeScriptKey: 1 byte(prefix) + 32 bytes(code_hash) + 1 byte(hash_type) + ? bytes(args) + 8 bytes(block_number) + 4 bytes(tx_index) + 4 bytes(io_index) + 1 byte(io_type)
        let mut start_key = vec![prefix as u8];
        start_key.extend_from_slice(script.code_hash().as_slice());
        start_key.extend_from_slice(script.hash_type().as_slice());
        start_key.extend_from_slice(script.args().as_slice());
        let from_block_slice = from_block.to_be_bytes();
        let to_block_slice = to_block.to_be_bytes();
        let iter = self.store.iter(&start_key, IteratorDirection::Forward)?;
        let tx_script_locations = iter
            .filter(move |(key, _)| key.starts_with(&start_key))
            .filter(move |(key, _)| {
                let block_number_slice = key[key.len() - 17..key.len() - 9].try_into();
                from_block_slice <= block_number_slice.unwrap()
                    && block_number_slice.unwrap() <= to_block_slice
            })
            .map(|(key, value)| {
                let block_number_slice = key[key.len() - 17..key.len() - 9].try_into();
                let tx_index_slice = key[key.len() - 9..key.len() - 5].try_into();
                let io_index_slice = key[key.len() - 5..key.len() - 1].try_into();
                let io_type_slice = key[key.len() - 1..].try_into();
                TxScriptLocation {
                    tx_hash: packed::Byte32::from_slice(&value).expect("stored tx hash"),
                    block_number: u64::from_be_bytes(block_number_slice.unwrap()),
                    tx_index: u32::from_be_bytes(tx_index_slice.unwrap()),
                    io_index: u32::from_be_bytes(io_index_slice.unwrap()),
                    io_type: u8::from_be_bytes(io_type_slice.unwrap()),
                }
            })
            .collect();
        Ok(tx_script_locations)
    }

    pub(crate) fn inner_get_transaction_history(
        &self,
        ident: String,
    ) -> Result<Vec<TransactionWithStatus>> {
        let mut ret = Vec::new();
        let address = parse_address(&ident)?;
        let tx_hashes = self.get_transactions_by_scripts(&address, vec![])?;
        let hash_clone = tx_hashes.clone();
        let txs_with_status = block_on!(self, get_transactions, hash_clone)?;

        for (index, item) in txs_with_status.into_iter().enumerate() {
            if let Some(tx) = item {
                ret.push(tx);
            } else {
                let tx_hash = tx_hashes.get(index).unwrap();
                return Err(
                    MercuryError::rpc(RpcError::CannotGetTxByHash(hex::encode(tx_hash))).into(),
                );
            }
        }

        Ok(ret)
    }

    pub(crate) fn get_transactions_by_script(
        &self,
        script: &packed::Script,
        prefix: indexer::KeyPrefix,
    ) -> Result<Vec<packed::Byte32>> {
        let mut start_key = vec![prefix as u8];
        start_key.extend_from_slice(&extract_raw_data(script));

        let iter = self.store.iter(&start_key, IteratorDirection::Forward)?;
        Ok(iter
            .take_while(|(key, _)| key.starts_with(&start_key))
            .map(|(_key, value)| packed::Byte32::from_slice(&value).expect("stored tx hash"))
            .collect())
    }

    fn acp_addr_to_secp(&self, addr: &Address) -> Address {
        Address::new(
            self.net_ty,
            AddressPayload::from_pubkey_hash(
                self.net_ty,
                H160::from_slice(&addr.payload().args()[0..20]).unwrap(),
            ),
        )
    }

    fn get_config(&self, script_name: &str) -> Result<packed::Script> {
        let ret = self
            .config
            .get(script_name)
            .cloned()
            .ok_or_else(|| MercuryError::rpc(RpcError::MissingConfig(script_name.to_string())))?
            .script;

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

    pub(crate) fn get_detailed_live_cell(
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

    fn get_all_udt_hashes(&self) -> Result<HashSet<Option<H256>>> {
        let prefix = *UDT_EXT_PREFIX;
        let mut start_key = prefix.to_vec();
        start_key.push(udt_balance::KeyPrefix::ScriptHash as u8);
        let prefix_len = start_key.len();

        let mut ret = self
            .store
            .iter(&start_key, IteratorDirection::Forward)?
            .take_while(|(key, _val)| key.starts_with(&start_key))
            .map(|(key, _val)| Some(H256::from_slice(&key.to_vec()[prefix_len..]).unwrap()))
            .collect::<HashSet<_>>();
        ret.insert(None);

        Ok(ret)
    }

    pub(crate) fn get_script_by_hash(&self, hash: [u8; 20]) -> Result<packed::Script> {
        let key = script_hash::Key::ScriptHash(hash).into_vec();
        let raw = self
            .store_get(*SCRIPT_HASH_EXT_PREFIX, key)?
            .ok_or_else(|| {
                MercuryError::rpc(RpcError::CannotGetScriptByHash(hex::encode(&hash)))
            })?;
        Ok(packed::Script::from_slice(&raw).unwrap())
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

fn order_then_paginate(
    tx_script_locations: Vec<TxScriptLocation>,
    order: OrderEnum,
    offset: u64,
    limit: u64,
) -> Vec<H256> {
    let mut hashes = match order {
        OrderEnum::Asc => {
            let mut hashes = tx_script_locations
                .iter()
                .sorted_by(|a, b| a.block_number.cmp(&b.block_number))
                .map(|item| item.tx_hash.unpack())
                .collect::<Vec<H256>>();
            // when a lock_script is used multiple times in the same transactions, it needs to dedup.
            hashes.dedup();
            hashes
        }
        OrderEnum::Desc => {
            let mut hashes = tx_script_locations
                .iter()
                .sorted_by(|a, b| b.block_number.cmp(&a.block_number))
                .map(|item| item.tx_hash.unpack())
                .collect::<Vec<H256>>();
            hashes.dedup();
            hashes
        }
    };
    let hashes_len = hashes.len();
    let start = offset as usize;
    let end = (offset + limit) as usize;
    if start > hashes_len {
        vec![]
    } else if start <= hashes_len && hashes_len < end {
        hashes.drain(start..hashes_len).collect()
    } else {
        hashes.drain(start..end).collect()
    }
}

#[cfg(test)]
mod tests {
    use jsonrpc_core::futures::future;
    use jsonrpc_http_server::tokio;

    #[tokio::test]
    async fn test_async_in_sync() {
        let (tx, rx) = crossbeam_channel::bounded(1);

        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let num = ready().await;
                tx.send(num).unwrap();
            })
        });

        let res = rx.recv().unwrap();
        assert_eq!(res, 0);
    }

    #[test]
    fn test_sync_in_async() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (tx, rx) = crossbeam_channel::bounded(1);

            std::thread::spawn(move || {
                let mut rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let num = ready().await;
                    tx.send(num).unwrap();
                })
            });

            let res = rx.recv().unwrap();
            assert_eq!(res, 0);
        })
    }

    async fn ready() -> u64 {
        future::ready(0).await
    }
}
