use crate::r#impl::{address_to_script, utils_types::*};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use common::hash::blake2b_160;
use common::utils::{
    decode_dao_block_number, decode_udt_amount, encode_udt_amount, parse_address, u256_low_u64,
};
use common::{
    Address, AddressPayload, Context, DetailedCell, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, PW_LOCK, SECP256K1, SUDT,
};
use common_logger::tracing_async;
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{
    MIN_CKB_CAPACITY, MIN_DAO_LOCK_PERIOD, STANDARD_SUDT_CAPACITY,
    WITHDRAWING_DAO_CELL_OCCUPIED_CAPACITY,
};
use core_rpc_types::lazy::{
    ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER, DAO_CODE_HASH,
    PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH, TX_POOL_CACHE,
};
use core_rpc_types::{
    decode_record_id, encode_record_id, AssetInfo, AssetType, Balance, DaoInfo, DaoState,
    ExtraFilter, ExtraType, HashAlgorithm, IOType, Identity, IdentityFlag, Item, JsonItem,
    Ownership, Record, SignAlgorithm, SignatureAction, SignatureInfo, SignatureLocation,
    SinceConfig, SinceFlag, SinceType, Source, Status,
};
use core_storage::{Storage, TransactionWrapper};

use ckb_dao_utils::extract_dao_data;
use ckb_types::core::{BlockNumber, Capacity, EpochNumberWithFraction, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256, U256};
use num_bigint::{BigInt, BigUint};
use num_traits::{ToPrimitive, Zero};

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::str::FromStr;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn get_script_builder(
        &self,
        script_name: &str,
    ) -> InnerResult<packed::ScriptBuilder> {
        Ok(self
            .builtin_scripts
            .get(script_name)
            .cloned()
            .ok_or_else(|| CoreError::MissingScriptInfo(script_name.to_string()))?
            .script
            .as_builder())
    }

    #[allow(clippy::unnecessary_unwrap)]
    #[tracing_async]
    pub(crate) async fn get_scripts_by_identity(
        &self,
        ctx: Context,
        ident: Identity,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut scripts = Vec::new();

        let (flag, pubkey_hash) = ident.parse()?;
        match flag {
            IdentityFlag::Ckb => {
                if lock_filter.is_none()
                    || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load()
                {
                    // get secp script
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    scripts.push(secp_script);
                }

                if lock_filter.is_none() || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load() {
                    let mut acp_scripts = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            (**ACP_CODE_HASH.load()).clone(),
                            Bytes::from(pubkey_hash.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;
                    scripts.append(&mut acp_scripts);
                }

                if lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load() {
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    let lock_hash: H256 = secp_script.calc_script_hash().unpack();
                    let lock_hash_160 = H160::from_slice(&lock_hash.0[0..20]).unwrap();

                    let mut receiver_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;

                    let mut sender_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (20, 40),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;

                    scripts.append(&mut receiver_cheque);
                    scripts.append(&mut sender_cheque);
                }
            }
            _ => {
                return Err(CoreError::UnsupportIdentityFlag.into());
            }
        }

        Ok(scripts)
    }

    #[tracing_async]
    pub(crate) async fn get_scripts_by_address(
        &self,
        _ctx: Context,
        addr: &Address,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut ret = Vec::new();
        let script = address_to_script(addr.payload());

        if (lock_filter.is_none() || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load())
            && self.is_script(&script, SECP256K1)?
        {
            ret.push(script.clone());
        }

        if (lock_filter.is_none() || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load())
            && self.is_script(&script, ACP)?
        {
            ret.push(script.clone());
        }

        if (lock_filter.is_none() || lock_filter.clone().unwrap() == **PW_LOCK_CODE_HASH.load())
            && self.is_script(&script, PW_LOCK)?
        {
            ret.push(script.clone());
        }

        if (lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load())
            && self.is_script(&script, CHEQUE)?
        {
            ret.push(script.clone());
        }

        Ok(ret)
    }

    pub(crate) fn get_secp_address_by_item(&self, item: Item) -> InnerResult<Address> {
        match item {
            Item::Address(address) => {
                let address = parse_address(&address)
                    .map_err(|err| CoreError::InvalidRpcParams(err.to_string()))?;
                let script = address_to_script(address.payload());
                if self.is_script(&script, SECP256K1)? {
                    Ok(address)
                } else if self.is_script(&script, ACP)? || self.is_script(&script, PW_LOCK)? {
                    let args: Bytes = address_to_script(address.payload()).args().unpack();
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(Bytes::from((&args[0..20]).to_vec()).pack())
                        .build();
                    Ok(self.script_to_address(&secp_script))
                } else {
                    Err(CoreError::UnsupportAddress.into())
                }
            }
            Item::Identity(identity) => match identity.flag()? {
                IdentityFlag::Ckb => {
                    let pubkey_hash = identity.hash();
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    Ok(self.script_to_address(&secp_script))
                }

                _ => Err(CoreError::UnsupportIdentityFlag.into()),
            },
            Item::Record(id) => {
                let (_out_point, ownership) = decode_record_id(id)?;
                match ownership {
                    Ownership::Address(address) => {
                        self.get_secp_address_by_item(Item::Address(address))
                    }
                    Ownership::LockHash(_lock_hash) => Err(CoreError::UnsupportOwnership.into()),
                }
            }
        }
    }

    #[tracing_async]
    pub(crate) async fn get_live_cells_by_item(
        &self,
        ctx: Context,
        item: Item,
        asset_infos: HashSet<AssetInfo>,
        tip_block_number: Option<BlockNumber>,
        tip_epoch_number: Option<RationalU256>,
        lock_filter: Option<H256>,
        extra: Option<ExtraType>,
        for_get_balance: bool,
        pagination: &mut PaginationRequest,
    ) -> InnerResult<Vec<DetailedCell>> {
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

        let mut ret = match item {
            Item::Identity(ident) => {
                let scripts = self
                    .get_scripts_by_identity(ctx.clone(), ident.clone(), lock_filter)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                if lock_hashes.is_empty() {
                    pagination.cursor = None;
                    return Ok(vec![]);
                }
                let cells = self
                    .get_live_cells(
                        ctx,
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        pagination.clone(),
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?;
                pagination.update_by_response(cells.clone());
                let (_flag, pubkey_hash) = ident.parse()?;
                let secp_lock_hash: H256 = self
                    .get_script_builder(SECP256K1)?
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
                let addr = Address::from_str(&addr).map_err(CoreError::ParseAddressError)?;
                let scripts = self
                    .get_scripts_by_address(ctx.clone(), &addr, lock_filter)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();

                if lock_hashes.is_empty() {
                    pagination.cursor = None;
                    return Ok(vec![]);
                }

                let cells = self
                    .get_live_cells(
                        ctx,
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        pagination.clone(),
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?;
                pagination.update_by_response(cells.clone());

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
                let (out_point, ownership) = decode_record_id(id)?;

                let scripts = match &ownership {
                    Ownership::Address(address) => {
                        let address = Address::from_str(address).map_err(CoreError::CommonError)?;
                        self.get_scripts_by_address(ctx.clone(), &address, lock_filter)
                            .await?
                    }

                    Ownership::LockHash(lock_hash) => {
                        let script_hash = H160::from_str(lock_hash)
                            .map_err(|e| CoreError::InvalidScriptHash(e.to_string()))?;
                        let script = self
                            .storage
                            .get_scripts(ctx.clone(), vec![script_hash], vec![], None, vec![])
                            .await
                            .map_err(|err| CoreError::DBError(err.to_string()))?
                            .get(0)
                            .cloned()
                            .ok_or(CoreError::CannotGetScriptByHash)?;
                        vec![script]
                    }
                };
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                if lock_hashes.is_empty() {
                    pagination.cursor = None;
                    return Ok(vec![]);
                }

                let cell = self
                    .get_live_cells(
                        ctx,
                        Some(out_point),
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        pagination.clone(),
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?;
                pagination.update_by_response(cell.clone());

                if !cell.response.is_empty() {
                    let cell = cell.response.get(0).cloned().unwrap();
                    let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

                    if code_hash == **CHEQUE_CODE_HASH.load() {
                        let secp_lock_hash: H160 = match &ownership {
                            Ownership::Address(address) => {
                                let address = parse_address(address)
                                    .map_err(|e| CoreError::CommonError(e.to_string()))?;

                                let lock_hash: H256 = address_to_script(address.payload())
                                    .calc_script_hash()
                                    .unpack();
                                H160::from_slice(&lock_hash.0[0..20]).unwrap()
                            }
                            Ownership::LockHash(lock_hash) => H160::from_str(lock_hash)
                                .map_err(|e| CoreError::InvalidScriptHash(e.to_string()))?,
                        };

                        let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();
                        let is_useful = if self.is_unlock(
                            EpochNumberWithFraction::from_full_value(cell.epoch_number)
                                .to_rational(),
                            tip_epoch_number.clone(),
                            self.cheque_timeout.clone(),
                        ) {
                            cell_args[20..40] == secp_lock_hash.0[0..20]
                        } else {
                            cell_args[0..20] == secp_lock_hash.0[0..20]
                        };

                        if is_useful || for_get_balance {
                            cells.push(cell);
                        }
                    } else if code_hash == **SECP256K1_CODE_HASH.load()
                        || code_hash == **ACP_CODE_HASH.load()
                    {
                        let record_address = match ownership {
                            Ownership::Address(address) => address,
                            Ownership::LockHash(_) => {
                                return Err(CoreError::InvalidRpcParams(
                                    "Nonexistent record id".to_string(),
                                )
                                .into());
                            }
                        };
                        if let Ok(record_address) = Address::from_str(&record_address) {
                            let record_lock: packed::Script = record_address.payload().into();
                            if record_lock == cell.cell_output.lock() {
                                cells.push(cell);
                            }
                        }
                    } else {
                        // todo: support more locks
                    }
                }

                cells
            }
        };

        if extra == Some(ExtraType::CellBase) {
            ret = ret.into_iter().filter(|cell| cell.tx_index == 0).collect();
        }
        Ok(ret)
    }

    #[tracing_async]
    async fn get_live_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<PaginationResponse<DetailedCell>> {
        let cells = if let Some(tip) = tip_block_number {
            let res = self
                .storage
                .get_historical_live_cells(ctx, lock_hashes, type_hashes, tip, out_point)
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?;

            PaginationResponse {
                response: res,
                next_cursor: None,
                count: None,
            }
        } else {
            self.storage
                .get_live_cells(
                    ctx,
                    out_point,
                    lock_hashes,
                    type_hashes,
                    block_range,
                    pagination,
                )
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?
        };

        Ok(cells)
    }

    #[tracing_async]
    pub(crate) async fn get_transactions_by_item(
        &self,
        ctx: Context,
        item: Item,
        asset_infos: HashSet<AssetInfo>,
        extra: Option<ExtraType>,
        range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<PaginationResponse<TransactionWrapper>> {
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
                let scripts = self
                    .get_scripts_by_identity(ctx.clone(), ident, None)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                self.storage
                    .get_transactions_by_scripts(
                        ctx.clone(),
                        lock_hashes,
                        type_hashes,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
            }

            Item::Address(addr) => {
                let addr =
                    parse_address(&addr).map_err(|e| CoreError::CommonError(e.to_string()))?;
                let scripts = self
                    .get_scripts_by_address(ctx.clone(), &addr, None)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<_>>();
                self.storage
                    .get_transactions_by_scripts(
                        ctx.clone(),
                        lock_hashes,
                        type_hashes,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
            }

            Item::Record(id) => {
                let (outpoint, _ownership) = decode_record_id(id)?;
                self.storage
                    .get_transactions(
                        ctx.clone(),
                        Some(outpoint),
                        vec![],
                        type_hashes,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
            }
        };

        if extra == Some(ExtraType::CellBase) {
            Ok(PaginationResponse {
                response: ret
                    .response
                    .into_iter()
                    .filter(|tx| tx.is_cellbase)
                    .collect(),
                next_cursor: ret.next_cursor,
                count: ret.count,
            })
        } else {
            Ok(ret)
        }
    }

    pub(crate) fn get_secp_lock_hash_by_item(&self, item: Item) -> InnerResult<H160> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse()?;
                match flag {
                    IdentityFlag::Ckb => {
                        let lock_hash: H256 = self
                            .get_script_builder(SECP256K1)?
                            .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                            .build()
                            .calc_script_hash()
                            .unpack();
                        Ok(H160::from_slice(&lock_hash.0[0..20]).unwrap())
                    }
                    _ => Err(CoreError::UnsupportIdentityFlag.into()),
                }
            }

            Item::Address(addr) => {
                let addr =
                    parse_address(&addr).map_err(|e| CoreError::CommonError(e.to_string()))?;
                let script = address_to_script(addr.payload());
                if self.is_script(&script, SECP256K1)?
                    || self.is_script(&script, ACP)?
                    || self.is_script(&script, PW_LOCK)?
                {
                    let lock_hash: H256 = self
                        .get_script_builder(SECP256K1)?
                        .args(Bytes::from(script.args().raw_data()[0..20].to_vec()).pack())
                        .build()
                        .calc_script_hash()
                        .unpack();
                    Ok(H160::from_slice(&lock_hash.0[0..20]).unwrap())
                } else {
                    Err(CoreError::UnsupportAddress.into())
                }
            }

            Item::Record(id) => {
                let (_, ownership) = decode_record_id(id)?;
                match ownership {
                    Ownership::Address(address) => {
                        Ok(self.get_secp_lock_hash_by_item(Item::Address(address))?)
                    }
                    Ownership::LockHash(lock_hash) => Ok(H160::from_str(&lock_hash)
                        .map_err(|e| CoreError::InvalidScriptHash(e.to_string()))?),
                }
            }
        }
    }

    pub(crate) fn get_secp_lock_args_by_item(&self, item: Item) -> InnerResult<H160> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse()?;
                match flag {
                    IdentityFlag::Ckb => Ok(pubkey_hash),
                    _ => Err(CoreError::UnsupportIdentityFlag.into()),
                }
            }

            Item::Address(addr) => {
                let addr =
                    parse_address(&addr).map_err(|e| CoreError::CommonError(e.to_string()))?;
                let script = address_to_script(addr.payload());
                if self.is_script(&script, SECP256K1)?
                    || self.is_script(&script, ACP)?
                    || self.is_script(&script, PW_LOCK)?
                {
                    let lock_args = script.args().raw_data();
                    Ok(H160::from_slice(&lock_args[0..20]).unwrap())
                } else {
                    Err(CoreError::UnsupportAddress.into())
                }
            }

            Item::Record(id) => {
                let (_, ownership) = decode_record_id(id)?;
                match ownership {
                    Ownership::Address(address) => {
                        Ok(self.get_secp_lock_hash_by_item(Item::Address(address))?)
                    }
                    Ownership::LockHash(lock_hash) => Ok(H160::from_str(&lock_hash)
                        .map_err(|e| CoreError::InvalidScriptHash(e.to_string()))?),
                }
            }
        }
    }

    fn is_in_cache(&self, cell: &packed::OutPoint) -> bool {
        let cache = TX_POOL_CACHE.read();
        cache.contains(cell)
    }

    #[allow(clippy::unnecessary_unwrap)]
    #[tracing_async]
    pub(crate) async fn to_record(
        &self,
        ctx: Context,
        cell: &DetailedCell,
        io_type: IOType,
        tip_block_number: Option<BlockNumber>,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<Vec<Record>> {
        let mut records = vec![];

        let block_number = cell.block_number;
        let epoch_number = cell.epoch_number;
        let udt_record = if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **SUDT_CODE_HASH.load() {
                let ownership = self
                    .generate_udt_ownership(ctx.clone(), cell, &io_type, tip_epoch_number.clone())
                    .await?;
                let id = encode_record_id(cell.out_point.clone(), ownership.clone());
                let asset_info = AssetInfo::new_udt(type_script.calc_script_hash().unpack());
                let status = self
                    .generate_udt_status(ctx.clone(), cell, &io_type, tip_epoch_number.clone())
                    .await?;
                let amount = self.generate_udt_amount(cell, &io_type);
                let extra = None;

                Some(Record {
                    id: hex::encode(&id),
                    ownership,
                    asset_info,
                    amount: amount.to_string(),
                    occupied: 0,
                    status,
                    extra,
                    block_number,
                    epoch_number,
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

        let ownership = self.generate_ckb_ownership(ctx.clone(), cell).await?;
        let id = encode_record_id(cell.out_point.clone(), ownership.clone());
        let asset_info = AssetInfo::new_ckb();
        let status = self.generate_ckb_status(cell);

        let amount = self.generate_ckb_amount(cell, &io_type);
        let extra = self
            .generate_extra(ctx.clone(), cell, io_type.clone(), tip_block_number)
            .await?;
        let data_occupied = Capacity::bytes(cell.cell_data.len())
            .map_err(|e| CoreError::OccupiedCapacityError(e.to_string()))?;
        let occupied = cell
            .cell_output
            .occupied_capacity(data_occupied)
            .map_err(|e| CoreError::OccupiedCapacityError(e.to_string()))?;

        let mut occupied = occupied.as_u64();
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        // To make CKB `free` represent available balance, pure ckb cell should be spendable.
        if cell.cell_data.is_empty()
            && cell.cell_output.type_().is_none()
            && lock_code_hash == **SECP256K1_CODE_HASH.load()
        {
            occupied = 0;
        }
        // secp sUDT cell with 0 udt amount should be spendable.
        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();
            if type_code_hash == **SUDT_CODE_HASH.load()
                && lock_code_hash == **SECP256K1_CODE_HASH.load()
                && self.generate_udt_amount(cell, &io_type).is_zero()
            {
                occupied = 0;
            }
        }

        let ckb_record = Record {
            id: hex::encode(&id),
            ownership,
            asset_info,
            amount: amount.to_string(),
            occupied,
            status,
            extra,
            block_number,
            epoch_number,
        };
        records.push(ckb_record);

        Ok(records)
    }

    #[tracing_async]
    pub(crate) async fn generate_ckb_ownership(
        &self,
        ctx: Context,
        cell: &DetailedCell,
    ) -> InnerResult<Ownership> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(Ownership::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let lock_hash =
                H160::from_slice(&cell.cell_output.lock().args().raw_data()[20..40].to_vec())
                    .unwrap();

            let res = self
                .storage
                .get_scripts(ctx, vec![lock_hash.clone()], vec![], None, vec![])
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?;
            if res.is_empty() {
                return Ok(Ownership::LockHash(lock_hash.to_string()));
            } else {
                return Ok(Ownership::Address(
                    self.script_to_address(res.get(0).unwrap()).to_string(),
                ));
            }
        }

        Ok(Ownership::Address(
            self.script_to_address(&cell.cell_output.lock()).to_string(),
        ))
    }

    fn generate_ckb_status(&self, cell: &DetailedCell) -> Status {
        Status::Fixed(cell.block_number)
    }

    fn generate_ckb_amount(&self, cell: &DetailedCell, io_type: &IOType) -> BigInt {
        let capacity: u64 = cell.cell_output.capacity().unpack();
        match io_type {
            IOType::Input => BigInt::from(capacity) * -1,
            IOType::Output => BigInt::from(capacity),
        }
    }

    #[tracing_async]
    async fn generate_udt_ownership(
        &self,
        ctx: Context,
        cell: &DetailedCell,
        io_type: &IOType,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<Ownership> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(Ownership::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let generate_epoch_num;
            let judge_epoch_num;

            if io_type == &IOType::Input {
                generate_epoch_num = self
                    .storage
                    .get_simple_transaction_by_hash(ctx.clone(), cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
                    .epoch_number;
                judge_epoch_num =
                    Some(EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational());
            } else {
                let res = self
                    .storage
                    .get_spent_transaction_hash(ctx.clone(), cell.out_point.clone())
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?;
                generate_epoch_num =
                    EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational();

                judge_epoch_num = if let Some(hash) = res {
                    let tx_info = self
                        .storage
                        .get_simple_transaction_by_hash(ctx.clone(), hash)
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;
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
                .get_scripts(ctx.clone(), vec![lock_hash.clone()], vec![], None, vec![])
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?;
            if res.is_empty() {
                return Ok(Ownership::LockHash(lock_hash.to_string()));
            } else {
                return Ok(Ownership::Address(
                    self.script_to_address(res.get(0).unwrap()).to_string(),
                ));
            }
        }

        Ok(Ownership::Address(
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

    #[tracing_async]
    async fn generate_udt_status(
        &self,
        ctx: Context,
        cell: &DetailedCell,
        io_type: &IOType,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<Status> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
            || lock_code_hash == **PW_LOCK_CODE_HASH.load()
        {
            let block_number = if io_type == &IOType::Input {
                self.storage
                    .get_simple_transaction_by_hash(ctx.clone(), cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
                    .block_number
            } else {
                cell.block_number
            };

            return Ok(Status::Fixed(block_number));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let res = self
                .storage
                .get_spent_transaction_hash(ctx.clone(), cell.out_point.clone())
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?;

            if let Some(hash) = res {
                let tx_info = self
                    .storage
                    .get_simple_transaction_by_hash(ctx.clone(), hash)
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?;
                Ok(Status::Fixed(tx_info.block_number))
            } else if self.is_unlock(
                EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational(),
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
            Err(CoreError::UnsupportLockScript(hex::encode(&lock_code_hash.0)).into())
        }
    }

    #[tracing_async]
    async fn generate_extra(
        &self,
        ctx: Context,
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
                        .get_simple_transaction_by_hash(
                            ctx.clone(),
                            cell.out_point.tx_hash().unpack(),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?
                        .block_number
                } else {
                    cell.block_number
                };

                let default_dao_data = Bytes::from(vec![0, 0, 0, 0, 0, 0, 0, 0]);
                let (state, start_hash, end_hash) = if cell.cell_data == default_dao_data {
                    let tip_hash = self
                        .storage
                        .get_canonical_block_hash(ctx.clone(), tip_block_number)
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;
                    (
                        DaoState::Deposit(block_num),
                        cell.block_hash.clone(),
                        tip_hash,
                    )
                } else {
                    let deposit_block_num = decode_dao_block_number(&cell.cell_data);
                    let tmp_hash = self
                        .storage
                        .get_canonical_block_hash(ctx.clone(), deposit_block_num)
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;
                    (
                        DaoState::Withdraw(deposit_block_num, block_num),
                        tmp_hash,
                        cell.block_hash.clone(),
                    )
                };

                let capacity: u64 = cell.cell_output.capacity().unpack();
                let reward = self
                    .calculate_maximum_withdraw(ctx.clone(), cell, start_hash, end_hash)
                    .await?
                    - capacity;

                return Ok(Some(ExtraFilter::Dao(DaoInfo { state, reward })));
            }

            let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            // If the cell is sUDT acp cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            if type_code_hash == **SUDT_CODE_HASH.load() && lock_code_hash == **ACP_CODE_HASH.load()
            {
                return Ok(None);
            }
            // If the cell is sUDT sepc cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            if type_code_hash == **SUDT_CODE_HASH.load()
                && lock_code_hash == **SECP256K1_CODE_HASH.load()
            {
                return Ok(None);
            }
            // TODO: If the cell is sUDT pw-lock cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            // if type_code_hash == **SUDT_CODE_HASH.load()
            //     && lock_code_hash == **PW_LOCK_CODE_HASH.load()
            // {
            //     return Ok(None);
            // }

            // Except sUDT acp cell and sUDT secp cell, cells with type setting can not spend its CKB.
            return Ok(Some(ExtraFilter::Freeze));
        } else if !cell.cell_data.is_empty() {
            // If cell data is not empty but type is empty which often used for storing contract binary,
            // the ckb amount of this record should not be spent.
            return Ok(Some(ExtraFilter::Freeze));
        }

        Ok(None)
    }

    /// Calculate maximum withdraw capacity of a deposited dao output
    #[tracing_async]
    pub async fn calculate_maximum_withdraw(
        &self,
        ctx: Context,
        cell: &DetailedCell,
        deposit_header_hash: H256,
        withdrawing_header_hash: H256,
    ) -> InnerResult<u64> {
        let deposit_header = self
            .storage
            .get_block_header(ctx.clone(), Some(deposit_header_hash), None)
            .await
            .map_err(|e| CoreError::DBError(e.to_string()))?;
        let withdrawing_header = self
            .storage
            .get_block_header(ctx.clone(), Some(withdrawing_header_hash), None)
            .await
            .map_err(|e| CoreError::DBError(e.to_string()))?;

        if deposit_header.number() >= withdrawing_header.number() {
            return Err(CoreError::InvalidOutPoint.into());
        }

        let (deposit_ar, _, _, _) = extract_dao_data(deposit_header.dao());
        let (withdrawing_ar, _, _, _) = extract_dao_data(withdrawing_header.dao());

        let occupied_capacity = WITHDRAWING_DAO_CELL_OCCUPIED_CAPACITY;
        let output_capacity: u64 = cell.cell_output.capacity().unpack();
        let counted_capacity = output_capacity
            .checked_sub(occupied_capacity)
            .ok_or(CoreError::Overflow)?;
        let withdraw_counted_capacity = BigUint::from(counted_capacity)
            * BigUint::from(withdrawing_ar)
            / BigUint::from(deposit_ar);
        let withdraw_counted_capacity: u64 = withdraw_counted_capacity
            .try_into()
            .map_err(|_e| CoreError::Overflow)?;
        let withdraw_capacity = withdraw_counted_capacity
            .checked_add(occupied_capacity)
            .ok_or(CoreError::Overflow)?;

        Ok(withdraw_capacity)
    }

    #[tracing_async]
    /// We do not use the accurate `occupied` definition in ckb, which indicates the capacity consumed for storage of the live cells.
    /// Because by this definition, `occupied` and `free` are both not good indicators for spendable balance.
    ///
    /// To make `free` represent spendable balance, We define `occupied`, `freezed` and `free` of CKBytes as following.
    /// `occupied`: the capacity consumed for storage, except pure CKB cell (cell_data and type are both empty). Pure CKB cell's `occupied` is zero.
    /// `freezed`: any cell which data or type is not empty, then its amount minus `occupied` is `freezed`. Except sUDT acp cell and sUDT secp cell which can be used to collect CKB in Mercury.
    /// `free`: amount minus `occupied` and `freezed`.
    pub(crate) async fn accumulate_balance_from_records(
        &self,
        ctx: Context,
        balances_map: &mut HashMap<(Ownership, AssetInfo), Balance>,
        records: &[Record],
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<()> {
        for record in records {
            let key = (record.ownership.clone(), record.asset_info.clone());

            let mut balance = match balances_map.get(&key) {
                Some(balance) => balance.clone(),
                None => Balance::new(record.ownership.clone(), record.asset_info.clone()),
            };

            let amount = u128::from_str(&record.amount).unwrap();
            let occupied = record.occupied as u128;
            let freezed = match &record.extra {
                Some(ExtraFilter::Dao(dao_info)) => match dao_info.state {
                    DaoState::Deposit(_) => amount - occupied,
                    DaoState::Withdraw(deposit_block_number, withdraw_block_number) => {
                        let deposit_epoch = self
                            .get_epoch_by_number(ctx.clone(), deposit_block_number)
                            .await?;
                        let withdraw_epoch = self
                            .get_epoch_by_number(ctx.clone(), withdraw_block_number)
                            .await?;
                        if is_dao_withdraw_unlock(
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
                    let epoch_number =
                        EpochNumberWithFraction::from_full_value(record.epoch_number).to_rational();
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

                Some(ExtraFilter::Freeze) => amount - occupied,

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

    #[tracing_async]
    pub(crate) async fn get_epoch_by_number(
        &self,
        ctx: Context,
        block_number: BlockNumber,
    ) -> InnerResult<RationalU256> {
        let header = self
            .storage
            .get_block_header(ctx, None, Some(block_number))
            .await
            .map_err(|_| CoreError::GetEpochFromNumberError(block_number))?;
        Ok(header.epoch().to_rational())
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
                EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational(),
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

    pub(crate) fn is_script(
        &self,
        script: &packed::Script,
        script_name: &str,
    ) -> InnerResult<bool> {
        let s = self
            .builtin_scripts
            .get(script_name)
            .cloned()
            .ok_or_else(|| CoreError::MissingScriptInfo(script_name.to_string()))?
            .script;
        Ok(script.code_hash() == s.code_hash() && script.hash_type() == s.hash_type())
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

    pub(crate) fn script_to_address(&self, script: &packed::Script) -> Address {
        let payload = AddressPayload::from_script(script);
        Address::new(self.network_type, payload, true)
    }

    fn is_cellbase_mature(&self, cell: &DetailedCell) -> bool {
        (**CURRENT_EPOCH_NUMBER.load()).clone().saturating_sub(
            EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational(),
        ) > self.cellbase_maturity
    }

    #[tracing_async]
    pub(crate) async fn balance_transfer_tx_capacity(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        transfer_components: &mut TransferComponents,
        pay_fee: Option<u64>,
        change: Option<String>,
    ) -> InnerResult<()> {
        // check inputs dup
        let mut input_cell_set: HashSet<packed::OutPoint> = transfer_components
            .inputs
            .iter()
            .map(|cell| cell.out_point.to_owned())
            .collect();
        if transfer_components.inputs.len() != input_cell_set.len() {
            return Err(CoreError::InvalidTxPrebuilt("duplicate inputs".to_string()).into());
        }

        // check current balance
        let inputs_capacity = transfer_components
            .inputs
            .iter()
            .map::<u64, _>(|cell| cell.cell_output.capacity().unpack())
            .sum::<u64>();
        let outputs_capacity = transfer_components
            .outputs
            .iter()
            .map::<u64, _>(|cell| cell.capacity().unpack())
            .sum::<u64>();
        let fee = if let Some(fee) = pay_fee { fee } else { 0 };
        let mut required_capacity = (outputs_capacity + fee) as i128
            - (inputs_capacity + transfer_components.dao_reward_capacity) as i128;
        if required_capacity.is_zero() {
            if pay_fee.is_none() {
                return Ok(());
            }
            if pay_fee.is_some() && transfer_components.fee_change_cell_index.is_some() {
                return Err(CoreError::InvalidTxPrebuilt("duplicate pool fee".to_string()).into());
            }
        }

        // when required_ckb > 0
        // balance capacity based on current tx
        // check outputs secp and acp belong to from
        for output_cell in &mut transfer_components.outputs {
            if required_capacity <= 0 {
                break;
            }

            if let Some((current_cell_capacity, cell_max_extra_capacity)) =
                self.caculate_current_and_extra_capacity(output_cell, &from_items)
            {
                if required_capacity >= cell_max_extra_capacity as i128 {
                    let new_output_cell = output_cell
                        .clone()
                        .as_builder()
                        .capacity((current_cell_capacity - cell_max_extra_capacity).pack())
                        .build();
                    *output_cell = new_output_cell;
                    required_capacity -= cell_max_extra_capacity as i128;
                } else {
                    let cell_extra_capacity =
                        u64::try_from(required_capacity).map_err(|_| CoreError::Overflow)?;
                    let new_output_cell = output_cell
                        .clone()
                        .as_builder()
                        .capacity((current_cell_capacity - cell_extra_capacity).pack())
                        .build();
                    *output_cell = new_output_cell;
                    required_capacity -= cell_extra_capacity as i128;
                }
            }
        }

        // when required_ckb > 0
        // balance capacity based on database
        // add new inputs
        let mut ckb_cells_cache = CkbCellsCache::new(from_items.clone());
        ckb_cells_cache
            .pagination
            .set_limit(Some(self.pool_cache_size));
        loop {
            if required_capacity <= 0 {
                break;
            }
            let (live_cell, asset_script_type) = self
                .pool_next_live_cell_for_capacity(
                    ctx.clone(),
                    &mut ckb_cells_cache,
                    required_capacity,
                )
                .await?;
            if self.is_in_cache(&live_cell.out_point) {
                continue;
            }
            if input_cell_set.contains(&live_cell.out_point) {
                continue;
            }
            input_cell_set.insert(live_cell.out_point.clone());
            let capacity_provided = self
                .add_live_cell_for_balance_capacity(
                    ctx.clone(),
                    live_cell,
                    asset_script_type,
                    required_capacity,
                    transfer_components,
                )
                .await;
            required_capacity -= capacity_provided as i128;
        }

        if required_capacity == 0 {
            if pay_fee.is_some() {
                for (index, output_cell) in transfer_components.outputs.iter_mut().enumerate() {
                    if self.is_acp_or_secp_belong_to_items(output_cell, &from_items) {
                        transfer_components.fee_change_cell_index = Some(index);
                        return Ok(());
                    }
                }
            } else {
                return Ok(());
            }
        }
        if required_capacity > 0 {
            return Err(CoreError::InvalidTxPrebuilt("balance fail".to_string()).into());
        }

        // change
        match change {
            None => {
                // change tx outputs secp cell and acp cell belong to from
                for (index, output_cell) in &mut transfer_components.outputs.iter_mut().enumerate()
                {
                    if self.is_acp_or_secp_belong_to_items(output_cell, &from_items) {
                        let current_capacity: u64 = output_cell.capacity().unpack();
                        let new_capacity =
                            u64::try_from(current_capacity as i128 - required_capacity)
                                .expect("impossible: overflow");
                        let new_output_cell = output_cell
                            .clone()
                            .as_builder()
                            .capacity(new_capacity.pack())
                            .build();
                        *output_cell = new_output_cell;
                        if pay_fee.is_some() {
                            transfer_components.fee_change_cell_index = Some(index);
                        }
                        return Ok(());
                    }
                }

                // change acp cell from db
                let mut cells_cache = AcpCellsCache::new(from_items.clone(), None);
                cells_cache.pagination.set_limit(Some(self.pool_cache_size));
                loop {
                    let ret = self
                        .poll_next_live_acp_cell(ctx.clone(), &mut cells_cache)
                        .await;
                    if let Ok(acp_cell) = ret {
                        if self.is_in_cache(&acp_cell.out_point) {
                            continue;
                        }
                        if input_cell_set.contains(&acp_cell.out_point) {
                            continue;
                        }
                        self.add_live_cell_for_balance_capacity(
                            ctx.clone(),
                            acp_cell,
                            AssetScriptType::ACP,
                            required_capacity,
                            transfer_components,
                        )
                        .await;
                        if pay_fee.is_some() {
                            transfer_components.fee_change_cell_index =
                                Some(transfer_components.outputs.len() - 1);
                        }
                        return Ok(());
                    }
                    break;
                }
            }
            Some(ref change_address) => {
                // change to tx outputs cell with same address
                for (index, output_cell) in transfer_components.outputs.iter_mut().enumerate() {
                    let cell_address = self.script_to_address(&output_cell.lock()).to_string();
                    if *change_address == cell_address {
                        let current_capacity: u64 = output_cell.capacity().unpack();
                        let new_capacity =
                            u64::try_from(current_capacity as i128 - required_capacity)
                                .expect("impossible: overflow");
                        let new_output_cell = output_cell
                            .clone()
                            .as_builder()
                            .capacity(new_capacity.pack())
                            .build();
                        *output_cell = new_output_cell;
                        if pay_fee.is_some() {
                            transfer_components.fee_change_cell_index = Some(index);
                        }
                        return Ok(());
                    }
                }
            }
        }

        // change new secp ckb cell to output, may need new live cells
        if required_capacity.unsigned_abs() < MIN_CKB_CAPACITY as u128 {
            loop {
                if required_capacity.unsigned_abs() >= MIN_CKB_CAPACITY as u128 {
                    break;
                }
                let (live_cell, asset_script_type) = self
                    .pool_next_live_cell_for_capacity(
                        ctx.clone(),
                        &mut ckb_cells_cache,
                        required_capacity,
                    )
                    .await?;
                if self.is_in_cache(&live_cell.out_point) {
                    continue;
                }
                if input_cell_set.contains(&live_cell.out_point) {
                    continue;
                }
                input_cell_set.insert(live_cell.out_point.clone());
                let capacity_provided = self
                    .add_live_cell_for_balance_capacity(
                        ctx.clone(),
                        live_cell,
                        asset_script_type,
                        required_capacity,
                        transfer_components,
                    )
                    .await;
                required_capacity -= capacity_provided as i128;
            }
        }
        let secp_address = match change {
            None => self.get_secp_address_by_item(from_items[0].clone())?,
            Some(change_address) => {
                let item = Item::Address(change_address);
                self.get_secp_address_by_item(item)?
            }
        };
        let change_capacity =
            u64::try_from(required_capacity.unsigned_abs()).expect("impossible: overflow");
        build_cell_for_output(
            change_capacity,
            secp_address.payload().into(),
            None,
            None,
            &mut transfer_components.outputs,
            &mut transfer_components.outputs_data,
        )?;
        if pay_fee.is_some() {
            transfer_components.fee_change_cell_index = Some(transfer_components.outputs.len() - 1);
        }

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn balance_transfer_tx_udt(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        asset_info: AssetInfo,
        source: Source,
        transfer_components: &mut TransferComponents,
    ) -> InnerResult<()> {
        // check inputs dup
        let mut input_cell_set: HashSet<packed::OutPoint> = transfer_components
            .inputs
            .iter()
            .map(|cell| cell.out_point.to_owned())
            .collect();
        if transfer_components.inputs.len() != input_cell_set.len() {
            return Err(CoreError::InvalidTxPrebuilt("duplicate inputs".to_string()).into());
        }

        // check current balance
        let inputs_udt_amount = transfer_components
            .inputs
            .iter()
            .map::<u128, _>(|cell| decode_udt_amount(&cell.cell_data))
            .sum::<u128>();
        let outputs_udt_amount = transfer_components
            .outputs_data
            .iter()
            .map::<u128, _>(|data| {
                let data: Bytes = data.unpack();
                decode_udt_amount(&data)
            })
            .sum::<u128>();
        let mut required_udt_amount =
            BigInt::from(outputs_udt_amount) - BigInt::from(inputs_udt_amount);
        let zero = BigInt::from(0);
        if required_udt_amount.is_zero() {
            return Ok(());
        }

        // when required_udt_amount > 0
        // balance udt amount based on database
        // add new inputs
        let mut udt_cells_cache =
            UdtCellsCache::new(from_items.clone(), asset_info.clone(), source.clone());
        udt_cells_cache
            .pagination
            .set_limit(Some(self.pool_cache_size));

        loop {
            if required_udt_amount <= zero {
                break;
            }
            let (live_cell, asset_script_type) = self
                .pool_next_live_cell_for_udt(
                    ctx.clone(),
                    &mut udt_cells_cache,
                    required_udt_amount.clone(),
                )
                .await?;
            if self.is_in_cache(&live_cell.out_point) {
                continue;
            }
            if input_cell_set.contains(&live_cell.out_point) {
                continue;
            }
            input_cell_set.insert(live_cell.out_point.clone());
            let udt_amount_provided = self
                .add_live_cell_for_balance_udt(
                    ctx.clone(),
                    live_cell,
                    asset_script_type,
                    required_udt_amount.clone(),
                    transfer_components,
                )
                .await?;
            required_udt_amount -= udt_amount_provided;
        }

        // udt change
        match source {
            Source::Free => {
                if required_udt_amount < zero {
                    return Err(CoreError::InvalidTxPrebuilt(
                        "impossile: change udt fail on Source::Free".to_string(),
                    )
                    .into());
                }
            }
            Source::Claimable => {
                let last_input_cell = transfer_components
                    .inputs
                    .last()
                    .expect("impossible: get last input fail");
                let receiver_address = match self
                    .generate_udt_ownership(ctx.clone(), last_input_cell, &IOType::Input, None)
                    .await?
                {
                    Ownership::Address(address) => address,
                    Ownership::LockHash(_) => return Err(CoreError::CannotFindAddressByH160.into()),
                };

                // find acp
                if required_udt_amount < zero {
                    let mut cells_cache = AcpCellsCache::new(
                        vec![Item::Identity(address_to_identity(&receiver_address)?)],
                        Some(asset_info.clone()),
                    );
                    cells_cache.pagination.set_limit(Some(self.pool_cache_size));
                    loop {
                        let ret = self
                            .poll_next_live_acp_cell(ctx.clone(), &mut cells_cache)
                            .await;
                        if let Ok(acp_cell) = ret {
                            if self.is_in_cache(&acp_cell.out_point) {
                                continue;
                            }
                            if input_cell_set.contains(&acp_cell.out_point) {
                                continue;
                            }
                            let udt_amount_provided = self
                                .add_live_cell_for_balance_udt(
                                    ctx.clone(),
                                    acp_cell,
                                    AssetScriptType::ACP,
                                    required_udt_amount.clone(),
                                    transfer_components,
                                )
                                .await?;
                            required_udt_amount -= udt_amount_provided;
                        }
                        break;
                    }
                }

                // new output secp udt cell
                if required_udt_amount < zero {
                    let change_udt_amount = required_udt_amount
                        .to_i128()
                        .expect("impossible: to i128 fail")
                        .unsigned_abs();
                    let type_script = self
                        .build_sudt_type_script(
                            ctx.clone(),
                            common::hash::blake2b_256_to_160(&asset_info.udt_hash),
                        )
                        .await?;
                    let secp_address =
                        self.get_secp_address_by_item(Item::Address(receiver_address))?;
                    build_cell_for_output(
                        STANDARD_SUDT_CAPACITY,
                        secp_address.payload().into(),
                        Some(type_script),
                        Some(change_udt_amount),
                        &mut transfer_components.outputs,
                        &mut transfer_components.outputs_data,
                    )
                    .expect("impossible: build output cell fail");
                }
            }
        }

        Ok(())
    }

    pub async fn pool_next_live_cell_for_capacity(
        &self,
        ctx: Context,
        ckb_cells_cache: &mut CkbCellsCache,
        required_capacity: i128,
    ) -> InnerResult<(DetailedCell, AssetScriptType)> {
        loop {
            if let Some((cell, asset_script_type)) = ckb_cells_cache.cell_deque.pop_front() {
                return Ok((cell, asset_script_type));
            }

            if ckb_cells_cache.array_index >= ckb_cells_cache.item_category_array.len() {
                return Err(CoreError::CkbIsNotEnough(format!(
                    "shortage: {}, items: {:?}",
                    required_capacity, ckb_cells_cache.items
                ))
                .into());
            }

            let (item_index, category_index) =
                ckb_cells_cache.item_category_array[ckb_cells_cache.array_index];
            match category_index {
                PoolCkbCategory::DaoClaim => {
                    let mut asset_ckb_set = HashSet::new();
                    asset_ckb_set.insert(AssetInfo::new_ckb());
                    let from_item = ckb_cells_cache.items[item_index].clone();
                    let cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            from_item.clone(),
                            asset_ckb_set.clone(),
                            None,
                            None,
                            Some((**SECP256K1_CODE_HASH.load()).clone()),
                            Some(ExtraType::Dao),
                            false,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
                    let withdrawing_cells = cells
                        .into_iter()
                        .filter(|cell| {
                            cell.cell_data != Box::new([0u8; 8]).to_vec()
                                && cell.cell_data.len() == 8
                        })
                        .filter(|cell| {
                            EpochNumberWithFraction::from_full_value(cell.epoch_number)
                                .to_rational()
                                + U256::from(4u64)
                                < tip_epoch_number
                        })
                        .collect::<Vec<_>>();
                    let mut dao_cells = vec![];
                    if !withdrawing_cells.is_empty() {
                        for withdrawing_cell in withdrawing_cells {
                            // get deposit_cell
                            let withdrawing_tx = self
                                .inner_get_transaction_with_status(
                                    ctx.clone(),
                                    withdrawing_cell.out_point.tx_hash().unpack(),
                                )
                                .await?;
                            let withdrawing_tx_input_index: u32 =
                                withdrawing_cell.out_point.index().unpack(); // input deposite cell has the same index
                            let deposit_cell =
                                &withdrawing_tx.input_cells[withdrawing_tx_input_index as usize];

                            if is_dao_withdraw_unlock(
                                EpochNumberWithFraction::from_full_value(deposit_cell.epoch_number)
                                    .to_rational(),
                                EpochNumberWithFraction::from_full_value(
                                    withdrawing_cell.epoch_number,
                                )
                                .to_rational(),
                                Some((**CURRENT_EPOCH_NUMBER.load()).clone()),
                            ) {
                                dao_cells.push(withdrawing_cell)
                            }
                        }
                    }
                    let dao_cells = dao_cells
                        .into_iter()
                        .map(|cell| {
                            (
                                cell,
                                AssetScriptType::Dao(ckb_cells_cache.items[item_index].clone()),
                            )
                        })
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = dao_cells;
                }
                PoolCkbCategory::CellBase => {
                    let mut asset_ckb_set = HashSet::new();
                    asset_ckb_set.insert(AssetInfo::new_ckb());
                    let ckb_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            asset_ckb_set.clone(),
                            None,
                            None,
                            Some((**SECP256K1_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let cell_base_cells = ckb_cells
                        .clone()
                        .into_iter()
                        .filter(|cell| cell.tx_index.is_zero() && self.is_cellbase_mature(cell))
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    let mut normal_ckb_cells = ckb_cells
                        .into_iter()
                        .filter(|cell| !cell.tx_index.is_zero() && cell.cell_data.is_empty())
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = cell_base_cells;
                    ckb_cells_cache.cell_deque.append(&mut normal_ckb_cells);
                }
                PoolCkbCategory::NormalSecp => {
                    // database query optimization: when priority CellBase and NormalSecp are next to each other
                    // database queries can be combined
                }
                PoolCkbCategory::SecpUdt => {
                    let secp_udt_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            HashSet::new(),
                            None,
                            None,
                            Some((**SECP256K1_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let secp_udt_cells = secp_udt_cells
                        .into_iter()
                        .filter(|cell| {
                            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                                let type_code_hash: H256 = type_script.code_hash().unpack();
                                type_code_hash == **SUDT_CODE_HASH.load()
                            } else {
                                false
                            }
                        })
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = secp_udt_cells;
                }
                PoolCkbCategory::Acp => {
                    let acp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            HashSet::new(),
                            None,
                            None,
                            Some((**ACP_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let acp_cells = acp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = acp_cells;
                }
            }
            if ckb_cells_cache.pagination.cursor.is_none() {
                ckb_cells_cache.array_index += 1;
            }
        }
    }

    pub async fn pool_next_live_cell_for_udt(
        &self,
        ctx: Context,
        udt_cells_cache: &mut UdtCellsCache,
        required_udt_amount: BigInt,
    ) -> InnerResult<(DetailedCell, AssetScriptType)> {
        let mut asset_udt_set = HashSet::new();
        asset_udt_set.insert(udt_cells_cache.asset_info.clone());

        loop {
            if let Some((cell, asset_script_type)) = udt_cells_cache.cell_deque.pop_front() {
                return Ok((cell, asset_script_type));
            }

            if udt_cells_cache.array_index >= udt_cells_cache.item_category_array.len() {
                return Err(CoreError::UDTIsNotEnough(format!(
                    "shortage: {}",
                    required_udt_amount
                ))
                .into());
            }

            let (item_index, category_index) =
                udt_cells_cache.item_category_array[udt_cells_cache.array_index];
            match category_index {
                PoolUdtCategory::ChequeInTime => {
                    let item_lock_hash =
                        self.get_secp_lock_hash_by_item(udt_cells_cache.items[item_index].clone())?;
                    let receiver_addr = self
                        .get_secp_address_by_item(udt_cells_cache.items[item_index].clone())?
                        .to_string();
                    let cheque_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            Some((**CHEQUE_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let cheque_cells_in_time = cheque_cells
                        .into_iter()
                        .filter(|cell| {
                            let receiver_lock_hash =
                                H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                                    .expect("impossible: get receiver lock hash fail");

                            receiver_lock_hash == item_lock_hash
                        })
                        .map(|cell| (cell, AssetScriptType::ChequeReceiver(receiver_addr.clone())))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = cheque_cells_in_time;
                }
                PoolUdtCategory::ChequeOutTime => {
                    let item_lock_hash =
                        self.get_secp_lock_hash_by_item(udt_cells_cache.items[item_index].clone())?;
                    let sender_addr = self
                        .get_secp_address_by_item(udt_cells_cache.items[item_index].clone())?
                        .to_string();
                    let cheque_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            Some((**CHEQUE_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let cheque_cells_time_out = cheque_cells
                        .into_iter()
                        .filter(|cell| {
                            let sender_lock_hash = H160::from_slice(
                                &cell.cell_output.lock().args().raw_data()[20..40],
                            )
                            .expect("impossible: get sender lock hash fail");
                            sender_lock_hash == item_lock_hash
                        })
                        .map(|cell| (cell, AssetScriptType::ChequeSender(sender_addr.clone())))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = cheque_cells_time_out;
                }
                PoolUdtCategory::SecpUdt => {
                    let secp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            Some((**SECP256K1_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let secp_cells = secp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = secp_cells;
                }
                PoolUdtCategory::Acp => {
                    let acp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            Some((**ACP_CODE_HASH.load()).clone()),
                            None,
                            false,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let acp_cells = acp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = acp_cells;
                }
            }
            if udt_cells_cache.pagination.cursor.is_none() {
                udt_cells_cache.array_index += 1;
            }
        }
    }

    pub async fn poll_next_live_acp_cell(
        &self,
        ctx: Context,
        acp_cells_cache: &mut AcpCellsCache,
    ) -> InnerResult<DetailedCell> {
        loop {
            if let Some(cell) = acp_cells_cache.cell_deque.pop_front() {
                return Ok(cell);
            }

            if acp_cells_cache.current_index >= acp_cells_cache.items.len() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let item = acp_cells_cache.items[acp_cells_cache.current_index].clone();
            let asset_infos = if let Some(asset_info) = acp_cells_cache.asset_info.clone() {
                let mut asset_udt_set = HashSet::new();
                asset_udt_set.insert(asset_info);
                asset_udt_set
            } else {
                HashSet::new()
            };
            let acp_cells = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    item.clone(),
                    asset_infos,
                    None,
                    None,
                    Some((**ACP_CODE_HASH.load()).clone()),
                    None,
                    false,
                    &mut acp_cells_cache.pagination,
                )
                .await?;
            let acp_cells = acp_cells.into_iter().collect::<VecDeque<_>>();
            acp_cells_cache.cell_deque = acp_cells;
            if acp_cells_cache.pagination.cursor.is_none() {
                acp_cells_cache.current_index += 1;
            }
        }
    }

    pub async fn add_live_cell_for_balance_capacity(
        &self,
        ctx: Context,
        cell: DetailedCell,
        asset_script_type: AssetScriptType,
        required_capacity: i128,
        transfer_components: &mut TransferComponents,
    ) -> i128 {
        let (addr, provided_capacity) = match asset_script_type {
            AssetScriptType::Secp256k1 => {
                let provided_capacity = if cell.cell_output.type_().is_none() {
                    transfer_components
                        .script_deps
                        .insert(SECP256K1.to_string());
                    let provided_capacity: u64 = cell.cell_output.capacity().unpack();
                    provided_capacity as i128
                } else {
                    let current_udt_amount = decode_udt_amount(&cell.cell_data);
                    if current_udt_amount.is_zero() {
                        transfer_components
                            .script_deps
                            .insert(SECP256K1.to_string());
                        transfer_components.script_deps.insert(SUDT.to_string());
                        let provided_capacity: u64 = cell.cell_output.capacity().unpack();
                        provided_capacity as i128
                    } else {
                        let current_capacity: u64 = cell.cell_output.capacity().unpack();
                        let max_provided_capacity =
                            current_capacity.saturating_sub(STANDARD_SUDT_CAPACITY);
                        let provided_capacity =
                            if required_capacity >= max_provided_capacity as i128 {
                                max_provided_capacity as i128
                            } else {
                                required_capacity
                            };

                        if provided_capacity.is_zero() {
                            return provided_capacity;
                        }

                        transfer_components
                            .script_deps
                            .insert(SECP256K1.to_string());
                        transfer_components.script_deps.insert(SUDT.to_string());
                        let outputs_capacity =
                            u64::try_from(current_capacity as i128 - provided_capacity)
                                .expect("impossible: overflow");
                        build_cell_for_output(
                            outputs_capacity,
                            cell.cell_output.lock(),
                            cell.cell_output.type_().to_opt(),
                            Some(current_udt_amount),
                            &mut transfer_components.outputs,
                            &mut transfer_components.outputs_data,
                        )
                        .expect("impossible: build output cell fail");
                        provided_capacity
                    }
                };
                let address = Address::new(
                    self.network_type,
                    AddressPayload::from(cell.cell_output.lock()),
                    true,
                )
                .to_string();
                (address, provided_capacity)
            }
            AssetScriptType::ACP => {
                let addr = Address::new(
                    self.network_type,
                    AddressPayload::from_pubkey_hash(
                        H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                            .unwrap(),
                    ),
                    true,
                )
                .to_string();
                let current_capacity: u64 = cell.cell_output.capacity().unpack();
                let current_udt_amount = decode_udt_amount(&cell.cell_data);
                let max_provided_capacity = current_capacity.saturating_sub(STANDARD_SUDT_CAPACITY);
                let provided_capacity = if required_capacity >= max_provided_capacity as i128 {
                    max_provided_capacity as i128
                } else {
                    required_capacity
                };

                if provided_capacity.is_zero() {
                    return provided_capacity;
                }

                transfer_components.script_deps.insert(ACP.to_string());
                transfer_components.script_deps.insert(SUDT.to_string());
                let outputs_capacity = u64::try_from(current_capacity as i128 - provided_capacity)
                    .expect("impossible: overflow");
                build_cell_for_output(
                    outputs_capacity,
                    cell.cell_output.lock(),
                    cell.cell_output.type_().to_opt(),
                    Some(current_udt_amount),
                    &mut transfer_components.outputs,
                    &mut transfer_components.outputs_data,
                )
                .expect("impossible: build output cell fail");
                (addr, provided_capacity)
            }
            AssetScriptType::Dao(from_item) => {
                // get deposit_cell
                let withdrawing_tx = self
                    .inner_get_transaction_with_status(
                        ctx.clone(),
                        cell.out_point.tx_hash().unpack(),
                    )
                    .await;
                let withdrawing_tx = if let Ok(withdrawing_tx) = withdrawing_tx {
                    withdrawing_tx
                } else {
                    return 0i128;
                };
                let withdrawing_tx_input_index: u32 = cell.out_point.index().unpack(); // input deposite cell has the same index
                let deposit_cell = &withdrawing_tx.input_cells[withdrawing_tx_input_index as usize];

                // calculate input since
                let unlock_epoch =
                    calculate_unlock_epoch_number(deposit_cell.epoch_number, cell.epoch_number);
                let since = if let Ok(since) = to_since(SinceConfig {
                    type_: SinceType::EpochNumber,
                    flag: SinceFlag::Absolute,
                    value: unlock_epoch,
                }) {
                    since
                } else {
                    return 0i128;
                };

                // calculate maximum_withdraw_capacity
                let maximum_withdraw_capacity = if let Ok(maximum_withdraw_capacity) = self
                    .calculate_maximum_withdraw(
                        ctx.clone(),
                        &cell,
                        deposit_cell.block_hash.clone(),
                        cell.block_hash.clone(),
                    )
                    .await
                {
                    maximum_withdraw_capacity
                } else {
                    return 0i128;
                };
                let cell_capacity: u64 = cell.cell_output.capacity().unpack();
                transfer_components.dao_reward_capacity +=
                    maximum_withdraw_capacity - cell_capacity;

                let from_address =
                    if let Ok(from_address) = self.get_secp_address_by_item(from_item) {
                        from_address
                    } else {
                        return 0i128;
                    };

                // add since
                transfer_components
                    .dao_since_map
                    .insert(transfer_components.inputs.len(), since);

                // build header deps
                let deposit_block_hash = deposit_cell.block_hash.pack();
                let withdrawing_block_hash = cell.block_hash.pack();
                if !transfer_components
                    .header_dep_map
                    .contains_key(&deposit_block_hash)
                {
                    transfer_components.header_dep_map.insert(
                        deposit_block_hash.clone(),
                        transfer_components.header_deps.len(),
                    );
                    transfer_components
                        .header_deps
                        .push(deposit_block_hash.clone());
                }
                if !transfer_components
                    .header_dep_map
                    .contains_key(&withdrawing_block_hash)
                {
                    transfer_components.header_dep_map.insert(
                        withdrawing_block_hash.clone(),
                        transfer_components.header_deps.len(),
                    );
                    transfer_components.header_deps.push(withdrawing_block_hash);
                }

                // fill type_witness_args
                let deposit_block_hash_index_in_header_deps = transfer_components
                    .header_dep_map
                    .get(&deposit_block_hash)
                    .expect("impossible: get header dep index failed")
                    .to_owned();
                let witness_args_input_type = Some(Bytes::from(
                    deposit_block_hash_index_in_header_deps
                        .to_le_bytes()
                        .to_vec(),
                ))
                .pack();
                transfer_components.type_witness_args.insert(
                    transfer_components.inputs.len(),
                    (witness_args_input_type, packed::BytesOpt::default()),
                );

                (from_address.to_string(), maximum_withdraw_capacity as i128)
            }
            _ => unreachable!(),
        };

        transfer_components.inputs.push(cell.clone());
        add_signature_action(
            addr,
            cell.cell_output.calc_lock_hash().to_string(),
            SignAlgorithm::Secp256k1,
            HashAlgorithm::Blake2b,
            &mut transfer_components.signature_actions,
            transfer_components.inputs.len() - 1,
        );

        provided_capacity
    }

    pub async fn add_live_cell_for_balance_udt(
        &self,
        ctx: Context,
        cell: DetailedCell,
        asset_script_type: AssetScriptType,
        required_udt_amount: BigInt,
        transfer_components: &mut TransferComponents,
    ) -> InnerResult<BigInt> {
        transfer_components.script_deps.insert(SUDT.to_string());
        let (address, provided_udt_amount) = match asset_script_type {
            AssetScriptType::ChequeReceiver(receiver_address) => {
                transfer_components.script_deps.insert(CHEQUE.to_string());

                let sender_address = match self.generate_ckb_ownership(ctx.clone(), &cell).await? {
                    Ownership::Address(address) => address,
                    Ownership::LockHash(_) => return Err(CoreError::CannotFindAddressByH160.into()),
                };
                let sender_address =
                    Address::from_str(&sender_address).map_err(CoreError::InvalidRpcParams)?;
                let sender_lock = address_to_script(sender_address.payload());
                build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    sender_lock,
                    None,
                    None,
                    &mut transfer_components.outputs,
                    &mut transfer_components.outputs_data,
                )?;

                (
                    receiver_address.clone(),
                    BigInt::from(decode_udt_amount(&cell.cell_data)),
                )
            }
            AssetScriptType::ChequeSender(sender_address) => {
                transfer_components.script_deps.insert(CHEQUE.to_string());

                let address = match self.generate_ckb_ownership(ctx.clone(), &cell).await? {
                    Ownership::Address(address) => address,
                    Ownership::LockHash(_) => return Err(CoreError::CannotFindAddressByH160.into()),
                };
                let address = Address::from_str(&address).map_err(CoreError::InvalidRpcParams)?;
                let sender_lock = address_to_script(address.payload());

                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data);
                let provided_udt_amount =
                    if required_udt_amount >= BigInt::from(max_provided_udt_amount) {
                        build_cell_for_output(
                            cell.cell_output.capacity().unpack(),
                            sender_lock,
                            None,
                            None,
                            &mut transfer_components.outputs,
                            &mut transfer_components.outputs_data,
                        )?;
                        BigInt::from(max_provided_udt_amount)
                    } else {
                        let outputs_udt_amount = (BigInt::from(max_provided_udt_amount)
                            - required_udt_amount.clone())
                        .to_u128()
                        .expect("impossible: overflow");
                        build_cell_for_output(
                            cell.cell_output.capacity().unpack(),
                            sender_lock,
                            cell.cell_output.type_().to_opt(),
                            Some(outputs_udt_amount),
                            &mut transfer_components.outputs,
                            &mut transfer_components.outputs_data,
                        )?;
                        required_udt_amount
                    };

                (sender_address.clone(), provided_udt_amount)
            }
            AssetScriptType::Secp256k1 => {
                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());

                let address = Address::new(
                    self.network_type,
                    AddressPayload::from(cell.cell_output.lock()),
                    true,
                )
                .to_string();
                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data);

                let provided_udt_amount =
                    if required_udt_amount >= BigInt::from(max_provided_udt_amount) {
                        build_cell_for_output(
                            cell.cell_output.capacity().unpack(),
                            cell.cell_output.lock(),
                            None,
                            None,
                            &mut transfer_components.outputs,
                            &mut transfer_components.outputs_data,
                        )?;
                        BigInt::from(max_provided_udt_amount)
                    } else {
                        let outputs_udt_amount = (BigInt::from(max_provided_udt_amount)
                            - required_udt_amount.clone())
                        .to_u128()
                        .expect("impossible: overflow");
                        build_cell_for_output(
                            cell.cell_output.capacity().unpack(),
                            cell.cell_output.lock(),
                            cell.cell_output.type_().to_opt(),
                            Some(outputs_udt_amount),
                            &mut transfer_components.outputs,
                            &mut transfer_components.outputs_data,
                        )?;
                        required_udt_amount
                    };

                (address, provided_udt_amount)
            }
            AssetScriptType::ACP => {
                let address = Address::new(
                    self.network_type,
                    AddressPayload::from_pubkey_hash(
                        H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                            .unwrap(),
                    ),
                    true,
                )
                .to_string();
                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data);
                let provided_udt_amount =
                    if required_udt_amount >= BigInt::from(max_provided_udt_amount) {
                        BigInt::from(max_provided_udt_amount)
                    } else {
                        required_udt_amount
                    };

                if provided_udt_amount.is_zero() {
                    return Ok(provided_udt_amount);
                }

                transfer_components.script_deps.insert(ACP.to_string());
                let outputs_udt_amount = (max_provided_udt_amount - provided_udt_amount.clone())
                    .to_u128()
                    .expect("impossible: overflow");
                build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    cell.cell_output.lock(),
                    cell.cell_output.type_().to_opt(),
                    Some(outputs_udt_amount),
                    &mut transfer_components.outputs,
                    &mut transfer_components.outputs_data,
                )?;

                (address, provided_udt_amount)
            }
            _ => unreachable!(),
        };

        transfer_components.inputs.push(cell.clone());
        add_signature_action(
            address,
            cell.cell_output.calc_lock_hash().to_string(),
            SignAlgorithm::Secp256k1,
            HashAlgorithm::Blake2b,
            &mut transfer_components.signature_actions,
            transfer_components.inputs.len() - 1,
        );

        Ok(provided_udt_amount)
    }

    pub fn caculate_current_and_extra_capacity(
        &self,
        cell: &packed::CellOutput,
        items: &[Item],
    ) -> Option<(u64, u64)> {
        if !self.is_acp_or_secp_belong_to_items(cell, items) {
            return None;
        }

        let address = self.script_to_address(&cell.lock()).to_string();
        let address = Address::from_str(&address).map_err(CoreError::CommonError);
        if let Ok(address) = address {
            if address.is_secp256k1() {
                if let Some(script) = cell.type_().to_opt() {
                    if let Ok(true) = self.is_script(&script, SUDT) {
                        let current_capacity: u64 = cell.capacity().unpack();
                        let extra_capacity =
                            current_capacity.saturating_sub(STANDARD_SUDT_CAPACITY);
                        Some((current_capacity, extra_capacity))
                    } else {
                        None
                    }
                } else {
                    let current_capacity: u64 = cell.capacity().unpack();
                    let extra_capacity = current_capacity.saturating_sub(MIN_CKB_CAPACITY);
                    Some((current_capacity, extra_capacity))
                }
            } else if address.is_acp() {
                let current_capacity: u64 = cell.capacity().unpack();
                let extra_capacity = current_capacity.saturating_sub(STANDARD_SUDT_CAPACITY);
                Some((current_capacity, extra_capacity))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn is_acp_or_secp_belong_to_items(&self, cell: &packed::CellOutput, items: &[Item]) -> bool {
        let cell_address = self.script_to_address(&cell.lock()).to_string();
        let item_of_cell = if let Ok(identity) = address_to_identity(&cell_address) {
            Item::Identity(identity)
        } else {
            return false;
        };
        let secp_address_of_cell = if let Ok(address) = self.get_secp_address_by_item(item_of_cell)
        {
            address
        } else {
            return false;
        };
        if let Some(type_script) = cell.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();
            if type_code_hash != **SUDT_CODE_HASH.load() {
                return false;
            }
        }
        for item in items {
            let ret = self.get_secp_address_by_item(item.to_owned());
            if let Ok(secp_address_of_item) = ret {
                if secp_address_of_item == secp_address_of_cell {
                    return true;
                }
            } else {
                continue;
            }
        }
        false
    }

    pub(crate) fn check_from_contain_to(
        &self,
        from_items: Vec<&JsonItem>,
        to_addresses: Vec<String>,
    ) -> InnerResult<()> {
        let mut from_secp_lock_args_set = HashSet::new();
        for json_item in from_items {
            let item = Item::try_from(json_item.to_owned())?;
            let args = self.get_secp_lock_args_by_item(item)?;
            from_secp_lock_args_set.insert(args);
        }
        for to_address in to_addresses {
            let to_item = Item::Identity(address_to_identity(&to_address)?);
            let to_secp_lock_args = self.get_secp_lock_args_by_item(to_item)?;
            if from_secp_lock_args_set.contains(&to_secp_lock_args) {
                return Err(CoreError::FromContainTo.into());
            }
        }
        Ok(())
    }
}

pub(crate) fn build_cell_for_output(
    capacity: u64,
    lock_script: packed::Script,
    type_script: Option<packed::Script>,
    udt_amount: Option<u128>,
    outputs: &mut Vec<packed::CellOutput>,
    cells_data: &mut Vec<packed::Bytes>,
) -> InnerResult<usize> {
    let cell_output = packed::CellOutputBuilder::default()
        .lock(lock_script)
        .type_(type_script.pack())
        .capacity(capacity.pack())
        .build();

    let cell_index = outputs.len();
    outputs.push(cell_output);

    let data: packed::Bytes = if let Some(udt_amount) = udt_amount {
        Bytes::from(encode_udt_amount(udt_amount)).pack()
    } else {
        Default::default()
    };
    cells_data.push(data);

    Ok(cell_index)
}

pub(crate) fn is_dao_withdraw_unlock(
    deposit_epoch: RationalU256,
    withdraw_epoch: RationalU256,
    tip_epoch: Option<RationalU256>,
) -> bool {
    let unlock_epoch = calculate_unlock_epoch(deposit_epoch, withdraw_epoch);
    if let Some(tip_epoch) = tip_epoch {
        tip_epoch >= unlock_epoch
    } else {
        *CURRENT_EPOCH_NUMBER.load().clone() >= unlock_epoch
    }
}

pub(crate) fn calculate_unlock_epoch(
    deposit_epoch: RationalU256,
    withdraw_epoch: RationalU256,
) -> RationalU256 {
    let cycle_count = calculate_dao_cycle_count(deposit_epoch.clone(), withdraw_epoch);
    let dao_cycle = RationalU256::from_u256(MIN_DAO_LOCK_PERIOD.into());
    deposit_epoch + dao_cycle * cycle_count
}

pub(crate) fn calculate_unlock_epoch_number(deposit_epoch: u64, withdraw_epoch: u64) -> u64 {
    let deposit_epoch = EpochNumberWithFraction::from_full_value(deposit_epoch);
    let deposit_epoch_rational_u256 = deposit_epoch.to_rational();
    let withdraw_epoch_rational_u256 =
        EpochNumberWithFraction::from_full_value(withdraw_epoch).to_rational();

    let cycle_count =
        calculate_dao_cycle_count(deposit_epoch_rational_u256, withdraw_epoch_rational_u256);
    let cycle_count = u256_low_u64(cycle_count.into_u256());

    EpochNumberWithFraction::new(
        deposit_epoch.number() + cycle_count * MIN_DAO_LOCK_PERIOD,
        deposit_epoch.index(),
        deposit_epoch.length(),
    )
    .full_value()
}

fn calculate_dao_cycle_count(
    deposit_epoch: RationalU256,
    withdraw_epoch: RationalU256,
) -> RationalU256 {
    let deposit_duration = withdraw_epoch - deposit_epoch;
    let dao_cycle = RationalU256::from_u256(MIN_DAO_LOCK_PERIOD.into());
    let mut cycle_count = deposit_duration / dao_cycle;
    let cycle_count_round_down = RationalU256::from_u256(cycle_count.clone().into_u256());
    if cycle_count_round_down < cycle_count {
        cycle_count = cycle_count_round_down + RationalU256::one();
    }
    cycle_count
}

pub fn add_signature_action(
    address: String,
    lock_hash: String,
    sign_algorithm: SignAlgorithm,
    hash_algorithm: HashAlgorithm,
    signature_actions: &mut HashMap<String, SignatureAction>,
    index: usize,
) {
    if let Some(entry) = signature_actions.get_mut(&lock_hash) {
        entry.add_group(index);
    } else {
        signature_actions.insert(
            lock_hash.clone(),
            SignatureAction {
                signature_location: SignatureLocation {
                    index,
                    offset: sign_algorithm.get_signature_offset().0,
                },
                signature_info: SignatureInfo {
                    algorithm: sign_algorithm,
                    address,
                },
                hash_algorithm,
                other_indexes_in_group: vec![],
            },
        );
    }
}

pub fn to_since(config: SinceConfig) -> InnerResult<u64> {
    let since = match (config.flag, config.type_) {
        (SinceFlag::Absolute, SinceType::BlockNumber) => 0b0000_0000u64,
        (SinceFlag::Relative, SinceType::BlockNumber) => 0b1000_0000u64,
        (SinceFlag::Absolute, SinceType::EpochNumber) => 0b0010_0000u64,
        (SinceFlag::Relative, SinceType::EpochNumber) => 0b1010_0000u64,
        (SinceFlag::Absolute, SinceType::Timestamp) => 0b0100_0000u64,
        (SinceFlag::Relative, SinceType::Timestamp) => 0b1100_0000u64,
    };
    if config.value > 0xff_ffff_ffff_ffffu64 {
        return Err(CoreError::InvalidRpcParams(
            "the value in the since config is too large".to_string(),
        )
        .into());
    }
    Ok((since << 56) + config.value)
}

pub fn build_cheque_args(receiver_address: Address, sender_address: Address) -> packed::Bytes {
    let mut ret = blake2b_160(address_to_script(receiver_address.payload()).as_slice()).to_vec();
    let sender = blake2b_160(address_to_script(sender_address.payload()).as_slice());
    ret.extend_from_slice(&sender);
    ret.pack()
}

pub fn address_to_identity(address: &str) -> InnerResult<Identity> {
    let address = Address::from_str(address).map_err(CoreError::CommonError)?;
    let script = address_to_script(address.payload());
    let pub_key_hash = if address.is_secp256k1() || address.is_acp() {
        script.args().as_slice()[4..24].to_vec()
    } else {
        return Err(
            CoreError::UnsupportLockScript(hex::encode(script.code_hash().as_slice())).into(),
        );
    };

    Ok(Identity::new(
        IdentityFlag::Ckb,
        H160::from_slice(&pub_key_hash).unwrap(),
    ))
}

pub(crate) fn check_same_enum_value(items: &[JsonItem]) -> InnerResult<()> {
    let all_items_is_same_variant = items.windows(2).all(|i| {
        matches!(
            (&i[0], &i[1]),
            (JsonItem::Identity(_), JsonItem::Identity(_))
                | (JsonItem::Address(_), JsonItem::Address(_))
                | (JsonItem::Record(_), JsonItem::Record(_))
        )
    });
    if all_items_is_same_variant {
        Ok(())
    } else {
        Err(CoreError::ItemsNotSameEnumValue.into())
    }
}

pub(crate) fn dedup_json_items(items: Vec<JsonItem>) -> Vec<JsonItem> {
    let mut items = items;
    items.sort_unstable();
    items.dedup();
    items
}

pub(crate) fn calculate_the_percentage(numerator: u64, denominator: u64) -> String {
    if denominator.is_zero() {
        "0.00000%".to_string()
    } else {
        let percentage = numerator as f64 / denominator as f64;
        format!("{:.5}%", 100.0 * percentage)
    }
}
