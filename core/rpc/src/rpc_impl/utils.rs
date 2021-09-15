use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER,
    DAO_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH, TX_POOL_CACHE,
};
use crate::types::{
    decode_record_id, encode_record_id, AddressOrLockHash, AssetInfo, AssetType, Balance, DaoInfo,
    DaoState, ExtraFilter, ExtraType, IOType, Identity, IdentityFlag, Item, Record, RequiredUDT,
    SignatureEntry, SignatureType, Source, Status, WitnessType,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_dao_block_number, decode_udt_amount, parse_address};
use common::{
    Address, AddressPayload, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1,
};
use core_storage::Storage;

use ckb_types::core::{
    BlockNumber, Capacity, EpochNumberWithFraction, RationalU256, TransactionView,
};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn get_script_builder(&self, script_name: &str) -> packed::ScriptBuilder {
        self.builtin_scripts
            .get(script_name)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    #[allow(clippy::unnecessary_unwrap)]
    pub(crate) async fn get_scripts_by_identity(
        &self,
        ident: Identity,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut scripts = Vec::new();

        let (flag, pubkey_hash) = ident.parse();
        match flag {
            IdentityFlag::Ckb => {
                if lock_filter.is_none()
                    || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load()
                {
                    // get secp script
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    scripts.push(secp_script);
                }

                if lock_filter.is_none() || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load() {
                    let mut acp_scripts = self
                        .storage
                        .get_scripts_by_partial_arg(
                            (**ACP_CODE_HASH.load()).clone(),
                            Bytes::from(pubkey_hash.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    scripts.append(&mut acp_scripts);
                }

                if lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load() {
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    let lock_hash: H256 = secp_script.calc_script_hash().unpack();
                    let lock_hash_160 = H160::from_slice(&lock_hash.0[0..20]).unwrap();

                    let mut receiver_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                    let mut sender_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (20, 40),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                    scripts.append(&mut receiver_cheque);
                    scripts.append(&mut sender_cheque);
                }
            }
            _ => {
                unreachable!();
            }
        }

        Ok(scripts)
    }

    pub(crate) async fn get_scripts_by_address(
        &self,
        addr: &Address,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut ret = Vec::new();
        let script = address_to_script(addr.payload());

        if (lock_filter.is_none() || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load())
            && self.is_script(&script, SECP256K1)
        {
            ret.push(script.clone());
        }

        if (lock_filter.is_none() || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load())
            && self.is_script(&script, ACP)
        {
            ret.push(script.clone());
        }

        if (lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load())
            && self.is_script(&script, CHEQUE)
        {
            let lock_hash: H256 = script.calc_script_hash().unpack();
            let lock_hash_160 = Bytes::from(lock_hash.0[0..20].to_vec());
            let mut cheque_with_receiver = self
                .storage
                .get_scripts_by_partial_arg(
                    (**CHEQUE_CODE_HASH.load()).clone(),
                    lock_hash_160.clone(),
                    (0, 20),
                )
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            let mut cheque_with_sender = self
                .storage
                .get_scripts_by_partial_arg(
                    (**CHEQUE_CODE_HASH.load()).clone(),
                    lock_hash_160,
                    (20, 40),
                )
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

            ret.append(&mut cheque_with_sender);
            ret.append(&mut cheque_with_receiver);
        }

        Ok(ret)
    }

    pub(crate) fn get_secp_address_by_item(&self, item: Item) -> InnerResult<Address> {
        match item {
            Item::Address(address) => {
                let address = parse_address(&address)
                    .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
                let script = address_to_script(address.payload());
                if self.is_script(&script, SECP256K1) {
                    Ok(address)
                } else if self.is_script(&script, ACP) {
                    let args: Bytes = address_to_script(address.payload()).args().unpack();
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from((&args[0..20]).to_vec()).pack())
                        .build();
                    Ok(self.script_to_address(&secp_script))
                } else {
                    // todo, return error in the future
                    unreachable!()
                }
            }
            Item::Identity(identity) => {
                match identity.flag() {
                    IdentityFlag::Ckb => {
                        let pubkey_hash = identity.hash();
                        let secp_script = self
                            .get_script_builder(SECP256K1)
                            .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                            .build();
                        Ok(self.script_to_address(&secp_script))
                    }
                    // todo, return error in the future
                    _ => unreachable!(),
                }
            }
            Item::Record(id) => {
                let (_out_point, address_or_lock_hash) = decode_record_id(id)?;
                match address_or_lock_hash {
                    AddressOrLockHash::Address(address) => {
                        self.get_secp_address_by_item(Item::Address(address))
                    }
                    AddressOrLockHash::LockHash(_lock_hash) => {
                        // todo, return error in the future
                        unreachable!()
                    }
                }
            }
        }
    }

    pub(crate) async fn get_live_cells_by_item(
        &self,
        item: Item,
        asset_infos: HashSet<AssetInfo>,
        tip_block_number: Option<BlockNumber>,
        tip_epoch_number: Option<RationalU256>,
        lock_filter: Option<H256>,
        extra: Option<ExtraFilter>,
    ) -> InnerResult<Vec<DetailedCell>> {
        let type_hashes = asset_infos
            .into_iter()
            .map(|asset_info| match asset_info.asset_type {
                AssetType::CKB => match extra {
                    Some(ExtraFilter::Dao(_)) => self
                        .builtin_scripts
                        .get(DAO)
                        .cloned()
                        .unwrap()
                        .script
                        .calc_script_hash()
                        .unpack(),
                    _ => H256::default(),
                },
                AssetType::UDT => asset_info.udt_hash,
            })
            .collect();

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self
                    .get_scripts_by_identity(ident.clone(), lock_filter)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                let cells = self
                    .get_live_cells(
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                let (_flag, pubkey_hash) = ident.parse();
                let secp_lock_hash: H256 = self
                    .get_script_builder(SECP256K1)
                    .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                    .build()
                    .calc_script_hash()
                    .unpack();

                cells
                    .response
                    .into_iter()
                    .filter(|cell| {
                        self.filter_useless_cheque(cell, &secp_lock_hash, tip_epoch_number.clone())
                    })
                    .collect()
            }

            Item::Address(addr) => {
                let addr = Address::from_str(&addr).unwrap();
                let scripts = self.get_scripts_by_address(&addr, lock_filter).await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                let cells = self
                    .get_live_cells(
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                cells
                    .response
                    .into_iter()
                    .filter(|cell| {
                        self.filter_useless_cheque(
                            cell,
                            &address_to_script(addr.payload())
                                .calc_script_hash()
                                .unpack(),
                            tip_epoch_number.clone(),
                        )
                    })
                    .collect()
            }

            Item::Record(id) => {
                let mut cells = vec![];
                let (out_point, address_or_lock_hash) = decode_record_id(id)?;
                let mut lock_hashes = vec![];
                if lock_filter.is_some() {
                    lock_hashes.push(lock_filter.unwrap());
                }

                let cell = self
                    .get_live_cells(
                        Some(out_point),
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                let cell = cell.response.get(0).cloned().unwrap();
                let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

                if code_hash == **CHEQUE_CODE_HASH.load() {
                    let secp_lock_hash: H160 = match address_or_lock_hash {
                        AddressOrLockHash::Address(address) => {
                            let address = parse_address(&address)
                                .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;

                            let lock_hash: H256 = address_to_script(address.payload())
                                .calc_script_hash()
                                .unpack();
                            H160::from_slice(&lock_hash.0[0..20]).unwrap()
                        }
                        AddressOrLockHash::LockHash(lock_hash) => {
                            H160::from_str(&lock_hash).unwrap()
                        }
                    };

                    let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();
                    let is_useful = if self.is_unlock(
                        RationalU256::from_u256(cell.epoch_number.clone()),
                        tip_epoch_number.clone(),
                        self.cheque_timeout.clone(),
                    ) {
                        cell_args[20..40] == secp_lock_hash.0[0..20]
                    } else {
                        cell_args[0..20] == secp_lock_hash.0[0..20]
                    };

                    if is_useful {
                        cells.push(cell);
                    }
                } else {
                    cells.push(cell);
                }

                cells
            }
        };

        if extra == Some(ExtraFilter::CellBase) {
            Ok(ret.into_iter().filter(|cell| cell.tx_index == 0).collect())
        } else {
            Ok(ret)
        }
    }

    async fn get_live_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<PaginationResponse<DetailedCell>> {
        let cells = if let Some(_tip_block_number) = tip_block_number {
            // todo: historical get_balance
            unimplemented!()
        } else {
            self.storage
                .get_live_cells(out_point, lock_hashes, type_hashes, block_range, pagination)
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
        };
        Ok(cells)
    }

    pub(crate) async fn get_transactions_by_item(
        &self,
        item: Item,
        asset_infos: HashSet<AssetInfo>,
        extra: Option<ExtraType>,
        range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<PaginationResponse<TransactionView>> {
        let type_hashes = asset_infos
            .into_iter()
            .map(|asset_info| match asset_info.asset_type {
                AssetType::CKB => match extra {
                    Some(ExtraType::Dao) => self
                        .builtin_scripts
                        .get(DAO)
                        .cloned()
                        .unwrap()
                        .script
                        .calc_script_hash()
                        .unpack(),
                    _ => H256::default(),
                },
                AssetType::UDT => asset_info.udt_hash,
            })
            .collect();

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self.get_scripts_by_identity(ident, None).await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                self.storage
                    .get_transactions(vec![], lock_hashes, type_hashes, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }

            Item::Address(addr) => {
                let addr = parse_address(&addr)
                    .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;
                let scripts = self.get_scripts_by_address(&addr, None).await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<_>>();
                self.storage
                    .get_transactions(vec![], lock_hashes, type_hashes, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }

            Item::Record(id) => {
                let (outpoint, _address_or_lock_hash) = decode_record_id(id)?;
                self.storage
                    .get_transactions(
                        vec![outpoint.tx_hash().unpack()],
                        vec![],
                        type_hashes,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }
        };

        if extra == Some(ExtraType::CellBase) {
            Ok(PaginationResponse {
                response: ret
                    .response
                    .into_iter()
                    .filter(|tx| tx.is_cellbase())
                    .collect(),
                next_cursor: ret.next_cursor,
                count: ret.count,
            })
        } else {
            Ok(ret)
        }
    }

    pub(crate) fn pool_asset(
        &self,
        pool_cells: &mut Vec<DetailedCell>,
        amount_required: &mut BigInt,
        resource_cells: Vec<DetailedCell>,
        is_ckb: bool,
        signature_entries: &mut HashMap<String, SignatureEntry>,
        script_type: AssetScriptType,
    ) -> bool {
        let zero = BigInt::from(0);
        for cell in resource_cells.iter() {
            if *amount_required <= zero {
                return true;
            }

            if self.is_in_cache(&cell.out_point) {
                continue;
            }

            let amount = if is_ckb {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity as u128
            } else {
                decode_udt_amount(&cell.cell_data)
            };

            *amount_required -= amount;

            let addr = match script_type {
                AssetScriptType::Secp256k1 => Address::new(
                    self.network_type,
                    AddressPayload::from(cell.cell_output.lock()),
                )
                .to_string(),
                AssetScriptType::ACP => Address::new(
                    self.network_type,
                    AddressPayload::from_pubkey_hash(
                        self.network_type,
                        H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                            .unwrap(),
                    ),
                )
                .to_string(),
                AssetScriptType::ChequeReceiver(ref s) => s.clone(),
                AssetScriptType::ChequeSender(ref s) => s.clone(),
                AssetScriptType::Dao => todo!(),
            };

            pool_cells.push(cell.clone());
            add_sig_entry(
                addr,
                cell.cell_output.calc_lock_hash().to_string(),
                signature_entries,
                pool_cells.len() - 1,
            );
        }

        *amount_required <= zero
    }

    pub(crate) fn get_secp_lock_hash_by_item(&self, item: Item) -> InnerResult<H160> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse();
                match flag {
                    IdentityFlag::Ckb => {
                        let lock_hash: H256 = self
                            .get_script_builder(SECP256K1)
                            .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                            .build()
                            .calc_script_hash()
                            .unpack();
                        Ok(H160::from_slice(&lock_hash.0[0..20]).unwrap())
                    }
                    _ => unreachable!(),
                }
            }

            Item::Address(addr) => {
                let addr = parse_address(&addr)
                    .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;
                let script = address_to_script(addr.payload());
                if self.is_script(&script, SECP256K1) || self.is_script(&script, ACP) {
                    let lock_hash: H256 = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(script.args().raw_data()[0..20].to_vec()).pack())
                        .build()
                        .calc_script_hash()
                        .unpack();
                    Ok(H160::from_slice(&lock_hash.0[0..20]).unwrap())
                } else {
                    unreachable!();
                }
            }

            Item::Record(id) => {
                let (_, address_or_lock_hash) = decode_record_id(id)?;
                match address_or_lock_hash {
                    AddressOrLockHash::Address(address) => {
                        Ok(self.get_secp_lock_hash_by_item(Item::Address(address))?)
                    }
                    AddressOrLockHash::LockHash(lock_hash) => {
                        Ok(H160::from_str(&lock_hash).unwrap())
                    }
                }
            }
        }
    }

    fn is_in_cache(&self, cell: &packed::OutPoint) -> bool {
        let cache = TX_POOL_CACHE.read();
        cache.contains(cell)
    }

    pub(crate) async fn to_record(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
        tip_block_number: Option<BlockNumber>,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<Vec<Record>> {
        let mut records = vec![];

        let udt_record = if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **SUDT_CODE_HASH.load() {
                let address_or_lock_hash = self
                    .generate_udt_address_or_lock_hash(cell, &io_type, tip_epoch_number.clone())
                    .await?;
                let id = encode_record_id(cell.out_point.clone(), address_or_lock_hash.clone());
                let asset_info = AssetInfo::new_udt(type_script.calc_script_hash().unpack());
                let status = self
                    .generate_udt_status(cell, &io_type, tip_epoch_number.clone())
                    .await?;
                let amount = self.generate_udt_amount(cell, &io_type);
                let extra = None;

                Some(Record {
                    id: hex::encode(&id),
                    address_or_lock_hash,
                    asset_info,
                    amount: amount.to_string(),
                    occupied: 0,
                    status,
                    extra,
                })
            } else {
                None
            }
        } else {
            None
        };

        if udt_record.is_some() {
            records.push(udt_record.unwrap());
        }

        let address_or_lock_hash = self.generate_ckb_address_or_lock_hash(cell).await?;
        let id = encode_record_id(cell.out_point.clone(), address_or_lock_hash.clone());
        let asset_info = AssetInfo::new_ckb();
        let status = self.generate_ckb_status(cell, &io_type);
        let amount = self.generate_ckb_amount(cell, &io_type);
        let extra = self.generate_extra(cell, io_type, tip_block_number).await?;
        let data_occupied = Capacity::bytes(cell.cell_data.len())
            .map_err(|e| RpcErrorMessage::OccupiedCapacityError(e.to_string()))?;
        let occupied = cell
            .cell_output
            .occupied_capacity(data_occupied)
            .map_err(|e| RpcErrorMessage::OccupiedCapacityError(e.to_string()))?;
        let ckb_record = Record {
            id: hex::encode(&id),
            address_or_lock_hash,
            asset_info,
            amount: amount.to_string(),
            occupied: occupied.as_u64(),
            status,
            extra,
        };
        records.push(ckb_record);

        Ok(records)
    }

    pub(crate) async fn generate_ckb_address_or_lock_hash(
        &self,
        cell: &DetailedCell,
    ) -> InnerResult<AddressOrLockHash> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(AddressOrLockHash::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let sender_lock_hash_160 = cell.cell_output.lock().args().raw_data()[20..40].to_vec();
            let lock_hash = H160::from_slice(&sender_lock_hash_160).unwrap();

            let res = self
                .storage
                .get_scripts(vec![lock_hash.clone()], vec![], None, vec![])
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if res.is_empty() {
                return Ok(AddressOrLockHash::LockHash(lock_hash.to_string()));
            } else {
                return Ok(AddressOrLockHash::Address(
                    self.script_to_address(res.get(0).unwrap()).to_string(),
                ));
            }
        }

        Ok(AddressOrLockHash::Address(
            self.script_to_address(&cell.cell_output.lock()).to_string(),
        ))
    }

    fn generate_ckb_status(&self, cell: &DetailedCell, io_type: &IOType) -> Status {
        let block_num = if io_type == &IOType::Input {
            0
        } else {
            cell.block_number
        };

        Status::Fixed(block_num)
    }

    fn generate_ckb_amount(&self, cell: &DetailedCell, io_type: &IOType) -> BigInt {
        let capacity: u64 = cell.cell_output.capacity().unpack();
        match io_type {
            IOType::Input => BigInt::from(capacity) * -1,
            IOType::Output => BigInt::from(capacity),
        }
    }

    async fn generate_udt_address_or_lock_hash(
        &self,
        cell: &DetailedCell,
        io_type: &IOType,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<AddressOrLockHash> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(AddressOrLockHash::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let generate_epoch_num;
            let judge_epoch_num;

            if io_type == &IOType::Input {
                generate_epoch_num = self
                    .storage
                    .get_simple_transaction_by_hash(cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .epoch_number;
                judge_epoch_num = Some(RationalU256::from_u256(cell.epoch_number.clone()));
            } else {
                let res = self
                    .storage
                    .get_spent_transaction_hash(cell.out_point.clone())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                generate_epoch_num = RationalU256::from_u256(cell.epoch_number.clone());

                judge_epoch_num = if let Some(hash) = res {
                    let tx_info = self
                        .storage
                        .get_simple_transaction_by_hash(hash)
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    Some(tx_info.epoch_number)
                } else {
                    tip_epoch_number.clone()
                };
            }

            let lock_hash_160 = if self.is_unlock(
                generate_epoch_num,
                judge_epoch_num,
                self.cheque_timeout.clone(),
            ) {
                cell.cell_output.lock().args().raw_data()[20..40].to_vec()
            } else {
                cell.cell_output.lock().args().raw_data()[0..20].to_vec()
            };
            let lock_hash = H160::from_slice(&lock_hash_160).unwrap();

            let res = self
                .storage
                .get_scripts(vec![lock_hash.clone()], vec![], None, vec![])
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if res.is_empty() {
                return Ok(AddressOrLockHash::LockHash(lock_hash.to_string()));
            } else {
                return Ok(AddressOrLockHash::Address(
                    self.script_to_address(res.get(0).unwrap()).to_string(),
                ));
            }
        }

        Ok(AddressOrLockHash::Address(
            self.script_to_address(&cell.cell_output.lock()).to_string(),
        ))
    }

    fn generate_udt_amount(&self, cell: &DetailedCell, io_type: &IOType) -> BigInt {
        let amount = BigInt::from(decode_udt_amount(&cell.cell_data));
        match io_type {
            IOType::Input => amount * -1,
            IOType::Output => amount,
        }
    }

    async fn generate_udt_status(
        &self,
        cell: &DetailedCell,
        io_type: &IOType,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<Status> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            let block_number = if io_type == &IOType::Input {
                self.storage
                    .get_simple_transaction_by_hash(cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .block_number
            } else {
                cell.block_number
            };

            return Ok(Status::Fixed(block_number));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let res = self
                .storage
                .get_spent_transaction_hash(cell.out_point.clone())
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if let Some(hash) = res {
                let tx_info = self
                    .storage
                    .get_simple_transaction_by_hash(hash)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                Ok(Status::Fixed(tx_info.block_number))
            } else if self.is_unlock(
                RationalU256::from_u256(cell.epoch_number.clone()),
                tip_epoch_number.clone(),
                self.cheque_timeout.clone(),
            ) {
                let mut timeout_block_num = cell.block_number;
                timeout_block_num += 180 * 6;

                Ok(Status::Fixed(timeout_block_num))
            } else {
                Ok(Status::Claimable(cell.block_number))
            }
        } else {
            Err(RpcErrorMessage::UnsupportUDTLockScript(hex::encode(
                &lock_code_hash.0,
            )))
        }
    }

    async fn generate_extra(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
        tip_block_number: Option<BlockNumber>,
    ) -> InnerResult<Option<ExtraFilter>> {
        let tip_block_number = tip_block_number.unwrap_or(**CURRENT_BLOCK_NUMBER.load());

        if cell.tx_index == 0 && io_type == IOType::Output {
            return Ok(Some(ExtraFilter::CellBase));
        }

        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **DAO_CODE_HASH.load() {
                let block_num = if io_type == IOType::Input {
                    self.storage
                        .get_simple_transaction_by_hash(cell.out_point.tx_hash().unpack())
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                        .block_number
                } else {
                    cell.block_number
                };

                let (state, start_hash, end_hash) = if cell.cell_data == vec![0, 0, 0, 0] {
                    let tip_hash = self
                        .storage
                        .get_canonical_block_hash(tip_block_number)
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    (
                        DaoState::Deposit(block_num),
                        cell.block_hash.clone(),
                        tip_hash,
                    )
                } else {
                    let deposit_block_num = decode_dao_block_number(&cell.cell_data);
                    let tmp_hash = self
                        .storage
                        .get_canonical_block_hash(deposit_block_num)
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    (
                        DaoState::Withdraw(deposit_block_num, block_num),
                        tmp_hash,
                        cell.block_hash.clone(),
                    )
                };

                let start_ar = self
                    .storage
                    .get_block_header(Some(start_hash), None)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .dao()
                    .raw_data()[8..15]
                    .to_vec();
                let start_ar = decode_dao_block_number(&start_ar);
                let end_ar = self
                    .storage
                    .get_block_header(Some(end_hash), None)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .dao()
                    .raw_data()[8..15]
                    .to_vec();
                let end_ar = decode_dao_block_number(&end_ar);

                let capacity: u64 = cell.cell_output.capacity().unpack();
                let reward = capacity * end_ar / start_ar - capacity;

                return Ok(Some(ExtraFilter::Dao(DaoInfo { state, reward })));
            }
        }
        Ok(None)
    }

    pub(crate) async fn pool_live_cells_by_items(
        &self,
        items: Vec<Item>,
        required_ckb: i64,
        required_udts: Vec<RequiredUDT>,
        source: Option<Source>,
        pool_cells: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_entries: &mut HashMap<String, SignatureEntry>,
    ) -> InnerResult<()> {
        let zero = BigInt::from(0);
        let mut asset_ckb_set = HashSet::new();

        for item in items.iter() {
            let item_lock_hash = self.get_secp_lock_hash_by_item(item.clone())?;
            self.pool_udt(
                &required_udts,
                item,
                source.clone(),
                pool_cells,
                item_lock_hash,
                script_set,
                signature_entries,
            )
            .await?;
        }

        let ckb_collect_already = pool_cells
            .iter()
            .map::<u64, _>(|cell| cell.cell_output.capacity().unpack())
            .sum::<u64>();
        let mut required_ckb = BigInt::from(required_ckb) - ckb_collect_already;

        if required_ckb <= zero {
            return Ok(());
        }

        asset_ckb_set.insert(AssetInfo::new_ckb());

        for item in items.iter() {
            let dao_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    Some(ExtraFilter::Dao(DaoInfo::new_deposit(0, 0))),
                )
                .await?;

            let dao_cells = dao_cells
                .into_iter()
                .filter(|cell| is_dao_unlock(cell))
                .collect::<Vec<_>>();

            if self.pool_asset(
                pool_cells,
                &mut required_ckb,
                dao_cells,
                true,
                signature_entries,
                AssetScriptType::Dao,
            ) {
                return Ok(());
            }

            let cell_base_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    Some(ExtraFilter::CellBase),
                )
                .await?;
            let cell_base_cells = cell_base_cells
                .into_iter()
                .filter(|cell| self.is_cellbase_mature(cell))
                .collect::<Vec<_>>();

            if self.pool_asset(
                pool_cells,
                &mut required_ckb,
                cell_base_cells,
                true,
                signature_entries,
                AssetScriptType::Secp256k1,
            ) {
                return Ok(());
            }

            let normal_ckb_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    None,
                )
                .await?;
            let normal_ckb_cells = normal_ckb_cells
                .into_iter()
                .filter(|cell| cell.cell_data.is_empty())
                .collect::<Vec<_>>();

            if self.pool_asset(
                pool_cells,
                &mut required_ckb,
                normal_ckb_cells,
                true,
                signature_entries,
                AssetScriptType::Secp256k1,
            ) {
                return Ok(());
            }

            if required_ckb > zero {
                return Err(RpcErrorMessage::TokenIsNotEnough(
                    AssetInfo::new_ckb().to_string(),
                ));
            }
        }

        Ok(())
    }

    pub(crate) async fn accumulate_balance_from_records(
        &self,
        balances_map: &mut HashMap<(AddressOrLockHash, AssetInfo), Balance>,
        records: &[Record],
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<()> {
        for record in records {
            let key = (
                record.address_or_lock_hash.clone(),
                record.asset_info.clone(),
            );

            let mut balance = match balances_map.get(&key) {
                Some(balance) => balance.clone(),
                None => Balance::new(
                    record.address_or_lock_hash.clone(),
                    record.asset_info.clone(),
                ),
            };

            let amount = u128::from_str(&record.amount).unwrap();
            let occupied = record.occupied as u128;
            let freezed = match &record.extra {
                Some(ExtraFilter::Dao(dao_info)) => match dao_info.state {
                    DaoState::Deposit(_) => amount - occupied,
                    DaoState::Withdraw(deposit_block_number, withdraw_block_number) => {
                        let deposit_epoch = self.get_epoch_by_number(deposit_block_number).await?;
                        let withdraw_epoch =
                            self.get_epoch_by_number(withdraw_block_number).await?;
                        if self.is_dao_withdraw_unlock(
                            deposit_epoch,
                            withdraw_epoch,
                            tip_epoch_number.clone(),
                        ) {
                            0u128
                        } else {
                            amount - occupied
                        }
                    }
                },
                Some(ExtraFilter::CellBase) => {
                    let block_number = match &record.status {
                        Status::Claimable(_) => unreachable!(),
                        Status::Fixed(block_number) => block_number,
                    };
                    let epoch_number = self.get_epoch_by_number(*block_number).await?;
                    if self.is_unlock(
                        epoch_number,
                        tip_epoch_number.clone(),
                        self.cellbase_maturity.clone(),
                    ) {
                        0u128
                    } else {
                        amount - occupied
                    }
                }
                None => 0u128,
            };
            let claimable = match &record.status {
                Status::Claimable(_) => amount,
                _ => 0u128,
            };
            let free = amount - occupied - freezed - claimable;

            let accumulate_occupied = occupied + u128::from_str(&balance.occupied).unwrap();
            let accumulate_freezed = freezed + u128::from_str(&balance.freezed).unwrap();
            let accumulate_claimable = claimable + u128::from_str(&balance.claimable).unwrap();
            let accumulate_free = free + u128::from_str(&balance.free).unwrap();

            balance.free = accumulate_free.to_string();
            balance.occupied = accumulate_occupied.to_string();
            balance.freezed = accumulate_freezed.to_string();
            balance.claimable = accumulate_claimable.to_string();

            balances_map.insert(key, balance.clone());
        }

        Ok(())
    }

    pub(crate) async fn get_epoch_by_number(
        &self,
        block_number: BlockNumber,
    ) -> InnerResult<RationalU256> {
        let header = self
            .storage
            .get_block_header(None, Some(block_number))
            .await
            .map_err(|_| RpcErrorMessage::GetEpochFromNumberError(block_number))?;
        Ok(header.epoch().to_rational())
    }

    async fn pool_udt(
        &self,
        required_udts: &[RequiredUDT],
        item: &Item,
        source: Option<Source>,
        pool_cells: &mut Vec<DetailedCell>,
        item_lock_hash: H160,
        script_set: &mut HashSet<String>,
        signature_entries: &mut HashMap<String, SignatureEntry>,
    ) -> InnerResult<()> {
        let zero = BigInt::from(0);
        for required_udt in required_udts.iter() {
            let asset_info = AssetInfo::new_udt(required_udt.udt_hash.clone());
            let mut asset_udt_set = HashSet::new();
            asset_udt_set.insert(asset_info.clone());
            let mut udt_required = BigInt::from(required_udt.amount_required);
            let cheque_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_udt_set.clone(),
                    None,
                    None,
                    Some((**CHEQUE_CODE_HASH.load()).clone()),
                    None,
                )
                .await?;

            if source.is_none() || source == Some(Source::Claimable) {
                let cheque_cells_in_time = cheque_cells
                    .clone()
                    .into_iter()
                    .filter(|cell| {
                        let receiver_lock_hash =
                            H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                                .unwrap();

                        receiver_lock_hash == item_lock_hash
                    })
                    .collect::<Vec<_>>();

                if !cheque_cells_in_time.is_empty() {
                    let receiver_addr = self
                        .storage
                        .get_registered_address(item_lock_hash.clone())
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                        .ok_or_else(|| {
                            RpcErrorMessage::LockHashIsNotRegistered(hex::encode(&item_lock_hash))
                        })?;

                    script_set.insert(CHEQUE.to_string());

                    if self.pool_asset(
                        pool_cells,
                        &mut udt_required,
                        cheque_cells_in_time,
                        false,
                        signature_entries,
                        AssetScriptType::ChequeReceiver(receiver_addr),
                    ) {
                        break;
                    }
                }
            }

            if source.is_none() || source == Some(Source::Free) {
                let cheque_cells_time_out = cheque_cells
                    .into_iter()
                    .filter(|cell| {
                        let sender_lock_hash =
                            H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                                .unwrap();
                        sender_lock_hash == item_lock_hash
                    })
                    .collect::<Vec<_>>();

                if !cheque_cells_time_out.is_empty() {
                    let sender_addr = self
                        .storage
                        .get_registered_address(item_lock_hash.clone())
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                        .ok_or_else(|| {
                            RpcErrorMessage::LockHashIsNotRegistered(hex::encode(&item_lock_hash))
                        })?;

                    script_set.insert(CHEQUE.to_string());
                    if self.pool_asset(
                        pool_cells,
                        &mut udt_required,
                        cheque_cells_time_out,
                        false,
                        signature_entries,
                        AssetScriptType::ChequeSender(sender_addr),
                    ) {
                        break;
                    }
                }

                let secp_cells = self
                    .get_live_cells_by_item(
                        item.clone(),
                        asset_udt_set.clone(),
                        None,
                        None,
                        Some((**SECP256K1_CODE_HASH.load()).clone()),
                        None,
                    )
                    .await?;

                if !secp_cells.is_empty() {
                    script_set.insert(SECP256K1.to_string());
                    if self.pool_asset(
                        pool_cells,
                        &mut udt_required,
                        secp_cells,
                        false,
                        signature_entries,
                        AssetScriptType::Secp256k1,
                    ) {
                        break;
                    }
                }

                let acp_cells = self
                    .get_live_cells_by_item(
                        item.clone(),
                        asset_udt_set.clone(),
                        None,
                        None,
                        Some((**ACP_CODE_HASH.load()).clone()),
                        None,
                    )
                    .await?;

                if !acp_cells.is_empty() {
                    script_set.insert(ACP.to_string());
                    if self.pool_asset(
                        pool_cells,
                        &mut udt_required,
                        acp_cells,
                        false,
                        signature_entries,
                        AssetScriptType::ACP,
                    ) {
                        break;
                    }
                }
            }

            if udt_required > zero {
                return Err(RpcErrorMessage::TokenIsNotEnough(asset_info.to_string()));
            }
        }

        Ok(())
    }

    fn filter_useless_cheque(
        &self,
        cell: &DetailedCell,
        secp_lock_hash: &H256,
        tip_epoch_number: Option<RationalU256>,
    ) -> bool {
        let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if code_hash == **CHEQUE_CODE_HASH.load() {
            let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();

            if self.is_unlock(
                RationalU256::from_u256(cell.epoch_number.clone()),
                tip_epoch_number,
                self.cheque_timeout.clone(),
            ) {
                cell_args[20..40] == secp_lock_hash.0[0..20]
            } else {
                cell_args[0..20] == secp_lock_hash.0[0..20]
            }
        } else {
            true
        }
    }

    pub(crate) fn is_script(&self, script: &packed::Script, script_name: &str) -> bool {
        let s = self
            .builtin_scripts
            .get(script_name)
            .cloned()
            .unwrap()
            .script;
        script.code_hash() == s.code_hash() && script.hash_type() == s.hash_type()
    }

    pub(crate) fn is_unlock(
        &self,
        from: RationalU256,
        end: Option<RationalU256>,
        unlock_gap: RationalU256,
    ) -> bool {
        if let Some(end) = end {
            end.saturating_sub(from) > unlock_gap
        } else {
            (**CURRENT_EPOCH_NUMBER.load()).clone().saturating_sub(from) > unlock_gap
        }
    }

    pub(crate) fn is_dao_withdraw_unlock(
        &self,
        deposit_epoch: RationalU256,
        withdraw_epoch: RationalU256,
        tip_epoch: Option<RationalU256>,
    ) -> bool {
        let deposit_duration = withdraw_epoch - deposit_epoch;
        let dao_cycle = RationalU256::from_u256(180u64.into());
        let cycle_count = deposit_duration / dao_cycle.clone();
        let unlock_epoch = dao_cycle * (cycle_count + RationalU256::one());

        if let Some(tip_epoch) = tip_epoch {
            tip_epoch > unlock_epoch
        } else {
            *CURRENT_EPOCH_NUMBER.load().clone() > unlock_epoch
        }
    }

    pub(crate) fn script_to_address(&self, script: &packed::Script) -> Address {
        let payload = AddressPayload::from_script(script, self.network_type);
        Address::new(self.network_type, payload)
    }

    fn is_cellbase_mature(&self, cell: &DetailedCell) -> bool {
        (**CURRENT_EPOCH_NUMBER.load())
            .clone()
            .saturating_sub_u256(cell.epoch_number.clone())
            > self.cellbase_maturity
    }
}

fn is_dao_unlock(_cell: &DetailedCell) -> bool {
    // todo: add check logic
    true
}

pub fn add_sig_entry(
    address: String,
    lock_hash: String,
    sigs_entry: &mut HashMap<String, SignatureEntry>,
    index: usize,
) {
    if let Some(entry) = sigs_entry.get_mut(&lock_hash) {
        entry.add_group();
    } else {
        sigs_entry.insert(
            lock_hash.clone(),
            SignatureEntry {
                type_: WitnessType::WitnessLock,
                group_len: 1,
                pub_key: address,
                signature_type: SignatureType::Secp256k1,
                index,
            },
        );
    }
}

pub enum AssetScriptType {
    Secp256k1,
    ACP,
    ChequeSender(String),
    ChequeReceiver(String),
    Dao,
}
