use crate::rpc_impl::{address_to_script, MercuryRpcImpl, USE_HEX_FORMAT};
use crate::types::{GetBalanceResponse, InnerCharge, QueryChargeResponse, ScriptType};
use crate::{error::RpcError, CkbRpc};

use common::utils::{decode_udt_amount, parse_address, to_fixed_array};
use common::{anyhow::Result, Address, MercuryError};
use core_extensions::{
    ckb_balance, lock_time, special_cells, udt_balance, DetailedCells, CKB_EXT_PREFIX,
    CURRENT_EPOCH, LOCK_TIME_PREFIX, SP_CELL_EXT_PREFIX, UDT_EXT_PREFIX,
};
use core_storage::{add_prefix, IteratorDirection, Store};

use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::core::{BlockNumber, BlockView, RationalU256};
use ckb_types::{packed, prelude::*, H160, H256};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

macro_rules! block_on {
    ($self_: ident, $func: ident $(, $arg: expr)*) => {{
        use jsonrpc_http_server::tokio::runtime;

        let (tx, rx) = crossbeam_channel::bounded(1);
        let client_clone = $self_.ckb_client.clone();

        std::thread::spawn(move || {
            let mut rt = runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let res = client_clone.$func($($arg),*).await;
                tx.send(res).unwrap();
            })
        });


        rx.recv()
            .map_err(|e| MercuryError::rpc(RpcError::ChannelError(e.to_string())))?
    }};
}

impl<S, C> MercuryRpcImpl<S, C>
where
    S: Store,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    pub(crate) fn inner_get_balance(
        &self,
        udt_hash: Option<H256>,
        addr: &Address,
    ) -> Result<GetBalanceResponse> {
        let sp_cells = self.get_sp_detailed_cells(addr)?;
        let unconstrained = self.get_unconstrained_balance(udt_hash.clone(), addr, &sp_cells)?;
        let locked = self.get_locked_balance(udt_hash.clone(), addr, &sp_cells)?;
        let fleeting = self.get_fleeting_balance(udt_hash, addr, &sp_cells)?;
        let res = GetBalanceResponse::new(unconstrained, fleeting, locked);
        Ok(res)
    }

    pub(crate) fn inner_scan_deposit(
        &self,
        block_number: BlockNumber,
        udt_hash: Option<H256>,
        idents: Vec<String>,
    ) -> Result<QueryChargeResponse> {
        let use_hex_format = USE_HEX_FORMAT.load();
        let block: BlockView =
            block_on!(self, get_block_by_number, block_number, **use_hex_format)?
                .ok_or_else(|| MercuryError::rpc(RpcError::CannotGetBlockByNumber(block_number)))?
                .into();

        if let Some(hash) = udt_hash {
            self.scan_block_udt(block, hash, idents)
        } else {
            self.scan_block_ckb(block, idents)
        }
    }

    pub(crate) fn scan_block_ckb(
        &self,
        block: BlockView,
        idents: Vec<String>,
    ) -> Result<QueryChargeResponse> {
        let secp256_lock = self.get_config(ckb_balance::SECP256K1_BLAKE160)?;
        let acp_lock = self.get_config(special_cells::ACP)?;
        let mut ret = idents
            .into_iter()
            .map(|addr| (parse_address(&addr).unwrap(), InnerCharge::new(addr)))
            .collect::<HashMap<_, _>>();

        // Skip cellbase when scan block
        for tx in block.transactions().iter().skip(1) {
            for output in tx.outputs().into_iter() {
                // Normal CKB cell condition.
                // Todo: Can do refactor here.
                if output.type_().is_none()
                    && output.lock().code_hash() == secp256_lock.code_hash()
                    && output.lock().hash_type() == secp256_lock.hash_type()
                {
                    let address = self.script_to_address(output.lock());
                    if ret.contains_key(&address) {
                        let capacity: u64 = output.capacity().unpack();
                        ret.get_mut(&address).unwrap().ckb_amount += capacity;
                    }
                }

                // ACP CKB cell condition.
                if output.type_().is_none()
                    && output.lock().code_hash() == acp_lock.code_hash()
                    && output.lock().hash_type() == acp_lock.hash_type()
                {
                    let args: Vec<u8> = output.lock().args().unpack();
                    let address = self.address_from_pubkey_hash(&args[0..20]);

                    if ret.contains_key(&address) {
                        let capacity: u64 = output.capacity().unpack();
                        ret.get_mut(&address).unwrap().ckb_amount += capacity;
                    }
                }
            }
        }

        Ok(QueryChargeResponse::new(ret.values().cloned().collect()))
    }

    pub(crate) fn scan_block_udt(
        &self,
        block: BlockView,
        udt_hash: H256,
        idents: Vec<String>,
    ) -> Result<QueryChargeResponse> {
        let secp256_lock = self.get_config(ckb_balance::SECP256K1_BLAKE160)?;
        let cheque_lock = self.get_config(special_cells::CHEQUE)?;
        let acp_lock = self.get_config(special_cells::ACP)?;
        let mut ret = idents
            .into_iter()
            .map(|addr| (parse_address(&addr).unwrap(), InnerCharge::new(addr)))
            .collect::<HashMap<_, _>>();

        // Skip cellbase when scan block
        for tx in block.transactions().iter().skip(1) {
            for out_point in tx.inputs().into_iter() {
                let cell = self
                    .get_detailed_live_cell(&out_point.previous_output())?
                    .ok_or_else(|| {
                        MercuryError::rpc(RpcError::CannotGetCellByOutPoint(
                            out_point.previous_output().to_string(),
                        ))
                    })?;

                // Claim cheque cell that address is receiver condition.
                if let Some(type_script) = cell.cell_output.type_().to_opt() {
                    if type_script.calc_script_hash().raw_data() == udt_hash.as_bytes()
                        && cell.cell_output.lock().code_hash() == cheque_lock.code_hash()
                        && cell.cell_output.lock().hash_type() == cheque_lock.hash_type()
                    {
                        let args: Vec<u8> = cell.cell_output.lock().args().unpack();
                        let address = self.address_from_pubkey_hash(&args[0..20]);

                        if ret.contains_key(&address) {
                            let udt_amount: u128 =
                                decode_udt_amount(&cell.cell_data.raw_data()[0..16]);
                            ret.get_mut(&address).unwrap().udt_amount += udt_amount;
                        }
                    }
                }
            }

            for (output, cell_data) in tx.outputs_with_data_iter() {
                if let Some(type_script) = output.type_().to_opt() {
                    if type_script.calc_script_hash().raw_data() == udt_hash.as_bytes() {
                        // Normal UDT cell condition, meanwhile add the cell capacity to the
                        // ckb amount.
                        if output.lock().code_hash() == secp256_lock.code_hash()
                            && output.lock().hash_type() == secp256_lock.hash_type()
                        {
                            let address = self.script_to_address(output.lock());
                            if ret.contains_key(&address) {
                                let capacity: u64 = output.capacity().unpack();
                                let udt_amount: u128 = decode_udt_amount(&cell_data[0..16]);
                                let val = ret.get_mut(&address).unwrap();
                                val.ckb_amount += capacity;
                                val.udt_amount += udt_amount;
                            }
                        }

                        // ACP UDT cell condition.
                        if output.lock().code_hash() == acp_lock.code_hash()
                            && output.lock().hash_type() == acp_lock.hash_type()
                        {
                            let args: Vec<u8> = output.lock().args().unpack();
                            let address = self.address_from_pubkey_hash(&args[0..20]);

                            if ret.contains_key(&address) {
                                let udt_amount: u128 = decode_udt_amount(&cell_data[0..16]);
                                ret.get_mut(&address).unwrap().udt_amount += udt_amount;
                            }
                        }
                    }
                }
            }
        }

        Ok(QueryChargeResponse::new(ret.values().cloned().collect()))
    }

    pub(crate) fn get_unconstrained_balance(
        &self,
        udt_hash: Option<H256>,
        addr: &Address,
        sp_cells: &DetailedCells,
    ) -> Result<u128> {
        if let Some(hash) = udt_hash {
            let unconstrained_udt_balance = self.udt_balance(addr, hash.clone())?.unwrap_or(0);
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
        let pubkey_hash = H160::from_slice(&script.args().raw_data()[0..20]).unwrap();
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
                lock_args.len() == 40 && lock_args[0..20] == pubkey_hash.0
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
        let script = address_to_script(addr.payload());
        let pubkey_hash = H160::from_slice(&script.args().raw_data()[0..20]).unwrap();
        let key = special_cells::Key::CkbAddress(&pubkey_hash);
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
        let mut ret = Vec::new();
        let mut lock_scripts = self
            .get_sp_cells_by_addr(&address)?
            .0
            .into_iter()
            .map(|cell| cell.cell_output.lock())
            .collect::<HashSet<_>>();
        lock_scripts.insert(address_to_script(address.payload()));

        for lock_script in lock_scripts.iter() {
            let mut hashes = self
                .get_transactions_by_script(lock_script, indexer::KeyPrefix::CellLockScript)?
                .into_iter()
                .map(|hash| hash.unpack())
                .collect::<Vec<H256>>();

            ret.append(&mut hashes);
        }

        Ok(ret)
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

    fn script_to_address(&self, script: packed::Script) -> Address {
        let payload = AddressPayload::from(script);
        Address::new(self.net_ty, payload)
    }

    fn address_from_pubkey_hash(&self, pubkey_hash: &[u8]) -> Address {
        assert!(pubkey_hash.len() == 20);
        Address::new(
            self.net_ty,
            AddressPayload::from_pubkey_hash(H160::from_slice(pubkey_hash).unwrap()),
        )
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
