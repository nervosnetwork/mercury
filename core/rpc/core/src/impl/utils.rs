use crate::r#impl::{address_to_script, utils_types::*};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use ckb_dao_utils::extract_dao_data;
use ckb_types::core::{BlockNumber, Capacity, EpochNumberWithFraction, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256, U256};
use common::address::{is_acp, is_pw_lock, is_secp256k1};
use common::hash::{blake2b_160, blake2b_256_to_160};
use common::lazy::{
    ACP_CODE_HASH, CHEQUE_CODE_HASH, DAO_CODE_HASH, PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH,
    SUDT_CODE_HASH,
};
use common::utils::{decode_dao_block_number, decode_udt_amount, encode_udt_amount, u256_low_u64};
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
use core_rpc_types::lazy::{CURRENT_EPOCH_NUMBER, TX_POOL_CACHE};
use core_rpc_types::{lazy::CURRENT_BLOCK_NUMBER, DaoInfo};
use core_rpc_types::{
    AssetInfo, AssetType, Balance, DaoState, ExtraFilter, ExtraType, IOType, Identity,
    IdentityFlag, Item, JsonItem, Record, SinceConfig, SinceFlag, SinceType,
};
use core_storage::{Storage, TransactionWrapper};
use num_bigint::{BigInt, BigUint};
use num_traits::{ToPrimitive, Zero};

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
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
        lock_filter: Option<&H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut scripts = Vec::new();

        let (flag, pubkey_hash) = ident.parse()?;
        match flag {
            IdentityFlag::Ckb => {
                if lock_filter.is_none() || lock_filter == SECP256K1_CODE_HASH.get() {
                    // get secp script
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(pubkey_hash.0.pack())
                        .build();
                    scripts.push(secp_script);
                }

                if lock_filter.is_none() || lock_filter == ACP_CODE_HASH.get() {
                    let mut acp_scripts = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            ACP_CODE_HASH.get().expect("get acp code hash"),
                            Bytes::from(pubkey_hash.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;
                    scripts.append(&mut acp_scripts);
                }

                if lock_filter.is_none() || lock_filter == CHEQUE_CODE_HASH.get() {
                    let secp_script = self
                        .get_script_builder(SECP256K1)?
                        .args(pubkey_hash.0.pack())
                        .build();
                    let lock_hash: H256 = secp_script.calc_script_hash().unpack();
                    let lock_hash_160 = blake2b_256_to_160(&lock_hash);

                    let mut receiver_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            CHEQUE_CODE_HASH.get().expect("get cheque code hash"),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;

                    let mut sender_cheque = self
                        .storage
                        .get_scripts_by_partial_arg(
                            ctx.clone(),
                            CHEQUE_CODE_HASH.get().expect("get cheque code hash"),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (20, 40),
                        )
                        .await
                        .map_err(|e| CoreError::DBError(e.to_string()))?;

                    scripts.append(&mut receiver_cheque);
                    scripts.append(&mut sender_cheque);
                }
            }
            IdentityFlag::Ethereum => {
                if lock_filter.is_none() || lock_filter == PW_LOCK_CODE_HASH.get() {
                    let pw_lock_script = self
                        .get_script_builder(PW_LOCK)?
                        .args(pubkey_hash.0.pack())
                        .build();
                    scripts.push(pw_lock_script);
                }
            }
            _ => {
                return Err(CoreError::UnsupportIdentityFlag.into());
            }
        }

        Ok(scripts)
    }

    pub(crate) fn get_scripts_by_address(
        &self,
        addr: &Address,
        lock_filter: Option<&H256>,
    ) -> Vec<packed::Script> {
        let mut ret = Vec::new();
        let script = address_to_script(addr.payload());
        if let Some(lock_filter) = lock_filter {
            let lock_hash: H256 = script.code_hash().unpack();
            if lock_hash != *lock_filter {
                return vec![];
            }
        }
        ret.push(script);
        ret
    }

    #[tracing_async]
    pub(crate) async fn get_live_cells_by_item(
        &self,
        ctx: Context,
        item: Item,
        asset_infos: HashSet<AssetInfo>,
        tip_block_number: Option<BlockNumber>,
        _tip_epoch_number: Option<RationalU256>,
        lock_filter: Option<&H256>,
        extra: Option<ExtraType>,
        pagination: &mut PaginationRequest,
    ) -> InnerResult<Vec<DetailedCell>> {
        let type_hashes = self.get_type_hashes(asset_infos, extra.clone());
        let (scripts, out_point) = match item {
            Item::Identity(ident) => (
                self.get_scripts_by_identity(ctx.clone(), ident, lock_filter)
                    .await?,
                None,
            ),
            Item::Address(addr) => {
                let addr = Address::from_str(&addr).map_err(CoreError::ParseAddressError)?;
                (self.get_scripts_by_address(&addr, lock_filter), None)
            }
            Item::OutPoint(out_point) => {
                let addr = self
                    .get_lock_by_out_point(out_point.to_owned().into())
                    .await
                    .map(|script| self.script_to_address(&script))?;
                (
                    self.get_scripts_by_address(&addr, lock_filter),
                    Some(out_point.into()),
                )
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
        let cell_page = self
            .get_live_cells(
                ctx,
                out_point,
                lock_hashes,
                type_hashes,
                tip_block_number,
                None,
                pagination.clone(),
            )
            .await
            .map_err(|e| CoreError::DBError(e.to_string()))?;
        pagination.update_by_response(cell_page.next_cursor.map(Into::into));

        let mut cells = cell_page.response;
        if extra == Some(ExtraType::Cellbase) {
            cells = cells
                .into_iter()
                .filter(|cell| cell.tx_index == 0)
                .collect();
        }
        Ok(cells)
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
            self.storage
                .get_historical_live_cells(
                    ctx,
                    lock_hashes,
                    type_hashes,
                    tip,
                    out_point,
                    pagination,
                )
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?
        } else {
            self.storage
                .get_live_cells(
                    ctx,
                    out_point,
                    lock_hashes,
                    type_hashes,
                    block_range,
                    None,
                    None,
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
        let limit_cellbase = extra == Some(ExtraType::Cellbase);
        let type_hashes = self.get_type_hashes(asset_infos, extra);

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
                        limit_cellbase,
                        pagination,
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
            }

            Item::Address(address) => {
                let address = Address::from_str(&address).map_err(CoreError::ParseAddressError)?;
                let scripts = self.get_scripts_by_address(&address, None);
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
                        limit_cellbase,
                        pagination,
                    )
                    .await
                    .map_err(|e| CoreError::DBError(e.to_string()))?
            }

            Item::OutPoint(out_point) => self
                .storage
                .get_transactions(
                    ctx.clone(),
                    Some(out_point.into()),
                    vec![],
                    type_hashes,
                    range,
                    limit_cellbase,
                    pagination,
                )
                .await
                .map_err(|e| CoreError::DBError(e.to_string()))?,
        };

        Ok(ret)
    }

    pub(crate) fn get_secp_lock_hash_by_pubkey_hash(&self, pubkey_hash: H160) -> InnerResult<H160> {
        let lock_hash: H256 = self
            .get_script_builder(SECP256K1)?
            .args(pubkey_hash.0.pack())
            .build()
            .calc_script_hash()
            .unpack();
        Ok(H160::from_slice(&lock_hash.0[0..20]).expect("impossible: build H160 fail"))
    }

    pub(crate) async fn get_default_owner_lock_by_item(
        &self,
        item: &Item,
    ) -> InnerResult<packed::Script> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse()?;
                match flag {
                    IdentityFlag::Ckb => Ok(self.get_builtin_script(SECP256K1, pubkey_hash)),
                    IdentityFlag::Ethereum => Ok(self.get_builtin_script(PW_LOCK, pubkey_hash)),
                    _ => Err(CoreError::UnsupportIdentityFlag.into()),
                }
            }

            Item::Address(address) => {
                let address = Address::from_str(address).map_err(CoreError::ParseAddressError)?;
                let script = address_to_script(address.payload());
                self.get_default_owner_lock_by_script(script)
            }

            Item::OutPoint(out_point) => {
                let lock = self
                    .get_lock_by_out_point(out_point.to_owned().into())
                    .await?;
                self.get_default_owner_lock_by_script(lock)
            }
        }
    }

    fn get_default_owner_lock_by_script(
        &self,
        script: packed::Script,
    ) -> InnerResult<packed::Script> {
        let lock_args = script.args().raw_data();
        if self.is_script(&script, SECP256K1)? || self.is_script(&script, ACP)? {
            let args = H160::from_slice(&lock_args[0..20]).expect("Impossible: parse args");
            Ok(self.get_builtin_script(SECP256K1, args))
        } else if self.is_script(&script, PW_LOCK)? {
            let args = H160::from_slice(&lock_args[0..20]).expect("Impossible: parse args");
            Ok(self.get_builtin_script(PW_LOCK, args))
        } else {
            Err(CoreError::UnsupportAddress.into())
        }
    }

    async fn get_lock_by_out_point(
        &self,
        out_point: packed::OutPoint,
    ) -> InnerResult<packed::Script> {
        let cells = self
            .storage
            .get_cells(
                Context::new(),
                Some(out_point),
                vec![],
                vec![],
                None,
                PaginationRequest::default(),
            )
            .await
            .map_err(|e| CoreError::DBError(e.to_string()))?;

        if cells.response.is_empty() {
            return Err(CoreError::CannotFindDetailedCellByOutPoint.into());
        }

        Ok(cells.response[0].cell_output.lock())
    }

    pub(crate) async fn get_default_owner_address_by_item(
        &self,
        item: &Item,
    ) -> InnerResult<Address> {
        self.get_default_owner_lock_by_item(item)
            .await
            .map(|script| self.script_to_address(&script))
    }

    pub(crate) async fn get_secp_address_by_item(&self, item: &Item) -> InnerResult<Address> {
        let address = self.get_default_owner_address_by_item(item).await?;
        if is_secp256k1(&address) {
            Ok(address)
        } else {
            Err(CoreError::UnsupportAddress.into())
        }
    }

    async fn get_a_change_address(
        &self,
        change_capacity: u64,
        items: &[Item],
        preferred_item_index: usize,
    ) -> InnerResult<Address> {
        let indexes: Vec<usize> = {
            let mut indexes: Vec<usize> = (0..items.len()).collect();
            if preferred_item_index < items.len() {
                let mut preferred = vec![preferred_item_index];
                preferred.append(&mut indexes);
                preferred
            } else {
                indexes
            }
        };
        for i in indexes {
            if let Ok(lock) = self.get_default_owner_lock_by_item(&items[i]).await {
                if change_capacity
                    >= calculate_cell_capacity(
                        &lock,
                        &packed::ScriptOpt::default(),
                        Capacity::bytes(0).expect("generate capacity"),
                    )
                {
                    return Ok(self.script_to_address(&lock));
                }
            }
        }
        Err(CoreError::UnsupportAddress.into())
    }

    pub(crate) async fn get_acp_address_by_item(&self, item: &Item) -> InnerResult<Address> {
        self.get_acp_lock_by_item(item)
            .await
            .map(|script| self.script_to_address(&script))
    }

    pub(crate) async fn get_acp_lock_by_item(&self, item: &Item) -> InnerResult<packed::Script> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse()?;
                match flag {
                    IdentityFlag::Ckb => Ok(self.get_builtin_script(ACP, pubkey_hash)),
                    IdentityFlag::Ethereum => Ok(self.get_builtin_script(PW_LOCK, pubkey_hash)),
                    _ => Err(CoreError::UnsupportIdentityFlag.into()),
                }
            }

            Item::Address(address) => {
                let address = Address::from_str(address).map_err(CoreError::ParseAddressError)?;
                self.get_acp_lock_by_address(address)
            }

            Item::OutPoint(out_point) => {
                let acp_lock = self
                    .get_lock_by_out_point(out_point.to_owned().into())
                    .await?;
                let address = self.script_to_address(&acp_lock);
                self.get_acp_lock_by_address(address)
            }
        }
    }

    fn get_acp_lock_by_address(&self, address: Address) -> InnerResult<packed::Script> {
        let script = address_to_script(address.payload());
        let lock_args = script.args().raw_data();
        if self.is_script(&script, SECP256K1)? || self.is_script(&script, ACP)? {
            let args = H160::from_slice(&lock_args[0..20]).expect("Impossible: parse args");
            Ok(self.get_builtin_script(ACP, args))
        } else if self.is_script(&script, PW_LOCK)? {
            let args = H160::from_slice(&lock_args[0..20]).expect("Impossible: parse args");
            Ok(self.get_builtin_script(PW_LOCK, args))
        } else {
            Err(CoreError::UnsupportAddress.into())
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
    ) -> InnerResult<Vec<Record>> {
        let mut records = vec![];

        let ownership = self.script_to_address(&cell.cell_output.lock());

        let block_number = cell.block_number;
        let epoch_number = cell.epoch_number;
        let udt_record = if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == *SUDT_CODE_HASH.get().expect("get sudt code hash") {
                let out_point = cell.out_point.to_owned().into();
                let asset_info = AssetInfo::new_udt(type_script.calc_script_hash().unpack());
                let amount = self.generate_udt_amount(cell);
                let extra = None;

                Some(Record {
                    out_point,
                    ownership: ownership.to_string(),
                    io_type: io_type.clone(),
                    asset_info,
                    amount: amount.into(),
                    occupied: 0.into(),
                    extra,
                    block_number: block_number.into(),
                    epoch_number: epoch_number.into(),
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(udt_record) = udt_record {
            records.push(udt_record);
        }

        let out_point = cell.out_point.to_owned().into();
        let asset_info = AssetInfo::new_ckb();

        let amount = self.generate_ckb_amount(cell) as u128;
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
        // To make CKB `free` represent available balance, pure ckb cell, acp cell/pw lock cell without type script should be spendable.
        if cell.cell_data.is_empty()
            && cell.cell_output.type_().is_none()
            && (Some(&lock_code_hash) == SECP256K1_CODE_HASH.get()
                || Some(&lock_code_hash) == ACP_CODE_HASH.get()
                || Some(&lock_code_hash) == PW_LOCK_CODE_HASH.get())
        {
            occupied = 0;
        }
        // secp sUDT cell with 0 udt amount should be spendable.
        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();
            if Some(&type_code_hash) == SUDT_CODE_HASH.get()
                && Some(&lock_code_hash) == SECP256K1_CODE_HASH.get()
                && self.generate_udt_amount(cell).is_zero()
            {
                occupied = 0;
            }
        }

        let ckb_record = Record {
            out_point,
            ownership: ownership.to_string(),
            io_type,
            asset_info,
            amount: amount.into(),
            occupied: occupied.into(),
            extra,
            block_number: block_number.into(),
            epoch_number: epoch_number.into(),
        };
        records.push(ckb_record);

        Ok(records)
    }

    #[tracing_async]
    pub(crate) async fn get_cheque_sender_address(
        &self,
        ctx: Context,
        cell: &DetailedCell,
    ) -> InnerResult<Address> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if Some(&lock_code_hash) == CHEQUE_CODE_HASH.get() {
            let lock_hash =
                H160::from_slice(&cell.cell_output.lock().args().raw_data()[20..40].to_vec())
                    .expect("get sender lock hash from cheque args");
            return self.get_address_by_lock_hash(ctx, lock_hash).await;
        }
        Err(CoreError::UnsupportLockScript("CHEQUE_CODE_HASH".to_string()).into())
    }

    fn generate_ckb_amount(&self, cell: &DetailedCell) -> u64 {
        cell.cell_output.capacity().unpack()
    }

    pub(crate) async fn get_cheque_receiver_address(
        &self,
        ctx: Context,
        cell: &DetailedCell,
    ) -> InnerResult<Address> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if Some(&lock_code_hash) == CHEQUE_CODE_HASH.get() {
            let lock_hash =
                H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20].to_vec())
                    .expect("get receiver lock hash from cheque args");
            return self.get_address_by_lock_hash(ctx, lock_hash).await;
        }
        Err(CoreError::UnsupportLockScript("CHEQUE_CODE_HASH".to_string()).into())
    }

    async fn get_address_by_lock_hash(
        &self,
        ctx: Context,
        lock_hash: H160,
    ) -> InnerResult<Address> {
        let res = self
            .storage
            .get_scripts(ctx, vec![lock_hash], vec![], None, vec![])
            .await
            .map_err(|e| CoreError::DBError(e.to_string()))?;
        if res.is_empty() {
            Err(CoreError::CannotFindAddressByH160.into())
        } else {
            Ok(self.script_to_address(res.get(0).unwrap()))
        }
    }

    fn generate_udt_amount(&self, cell: &DetailedCell) -> u128 {
        decode_udt_amount(&cell.cell_data).unwrap_or(0)
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
            return Ok(Some(ExtraFilter::Cellbase));
        }

        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if Some(&type_code_hash) == DAO_CODE_HASH.get() {
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
                        DaoState::Deposit(block_num.into()),
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
                        DaoState::Withdraw(deposit_block_num.into(), block_num.into()),
                        tmp_hash,
                        cell.block_hash.clone(),
                    )
                };

                let capacity: u64 = cell.cell_output.capacity().unpack();
                let reward = self
                    .calculate_maximum_withdraw(ctx.clone(), cell, start_hash, end_hash)
                    .await?
                    - capacity;

                return Ok(Some(ExtraFilter::Dao(DaoInfo {
                    state,
                    reward: reward.into(),
                })));
            }

            let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            // If the cell is sUDT acp cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            if Some(&type_code_hash) == SUDT_CODE_HASH.get()
                && Some(&lock_code_hash) == ACP_CODE_HASH.get()
            {
                return Ok(None);
            }
            // If the cell is sUDT sepc cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            if Some(&type_code_hash) == SUDT_CODE_HASH.get()
                && Some(&lock_code_hash) == SECP256K1_CODE_HASH.get()
            {
                return Ok(None);
            }
            // If the cell is sUDT pw-lock cell, as Mercury can collect CKB by it, so its ckb amount minus 'occupied' is spendable.
            if Some(&type_code_hash) == SUDT_CODE_HASH.get()
                && Some(&lock_code_hash) == PW_LOCK_CODE_HASH.get()
            {
                return Ok(None);
            }

            // Except sUDT acp cell, sUDT secp and sUDT pw lock cell, cells with type setting can not spend its CKB.
            return Ok(Some(ExtraFilter::Frozen));
        } else if !cell.cell_data.is_empty() {
            // If cell data is not empty but type is empty which often used for storing contract binary,
            // the ckb amount of this record should not be spent.
            return Ok(Some(ExtraFilter::Frozen));
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

        if deposit_header.number() > withdrawing_header.number() {
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
    /// To make `free` represent spendable balance, We define `occupied`, `frozen` and `free` of CKBytes as following.
    /// `occupied`: the capacity consumed for storage, except pure CKB cell (cell_data and type are both empty). Pure CKB cell's `occupied` is zero.
    /// `frozen`: any cell which data or type is not empty, then its amount minus `occupied` is `frozen`. Except sUDT acp cell, sUDT secp cell and sUDT pw lock cell which can be used to collect CKB in Mercury.
    /// `free`: amount minus `occupied` and `frozen`.
    pub(crate) async fn accumulate_balance_from_records(
        &self,
        ctx: Context,
        balances_map: &mut BTreeMap<(String, AssetInfo), Balance>,
        records: Vec<Record>,
        tip_epoch_number: Option<RationalU256>,
    ) -> InnerResult<()> {
        for record in records {
            let key = (record.ownership, record.asset_info);

            let mut balance = match balances_map.get(&key) {
                Some(balance) => balance.clone(),
                None => Balance::new(key.0.clone(), key.1.clone()),
            };

            let amount: u128 = record.amount.into();
            let occupied = {
                let occupied: u64 = record.occupied.into();
                occupied as u128
            };
            let frozen = match &record.extra {
                Some(ExtraFilter::Dao(dao_info)) => match dao_info.state {
                    DaoState::Deposit(_) => amount - occupied as u128,
                    DaoState::Withdraw(deposit_block_number, withdraw_block_number) => {
                        let deposit_epoch = self
                            .get_epoch_by_number(ctx.clone(), deposit_block_number.into())
                            .await?;
                        let withdraw_epoch = self
                            .get_epoch_by_number(ctx.clone(), withdraw_block_number.into())
                            .await?;
                        if is_dao_withdraw_unlock(
                            deposit_epoch,
                            withdraw_epoch,
                            tip_epoch_number.clone(),
                        ) {
                            0u128
                        } else {
                            amount - occupied as u128
                        }
                    }
                },

                Some(ExtraFilter::Cellbase) => {
                    let epoch_number =
                        EpochNumberWithFraction::from_full_value(record.epoch_number.into())
                            .to_rational();
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

                Some(ExtraFilter::Frozen) => amount - occupied,

                None => 0u128,
            };

            let free = amount - occupied - frozen;

            let accumulate_free: u128 = free + balance.free.value();
            let accumulate_occupied: u128 = occupied + balance.occupied.value();
            let accumulate_frozen: u128 = frozen + balance.frozen.value();

            balance.free = accumulate_free.into();
            balance.occupied = accumulate_occupied.into();
            balance.frozen = accumulate_frozen.into();

            balances_map.insert(key, balance);
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

    fn filter_cheque_cell(
        &self,
        item: &Item,
        cheque_cell: &DetailedCell,
        tip_epoch_number: Option<RationalU256>,
    ) -> bool {
        let code_hash: H256 = cheque_cell.cell_output.lock().code_hash().unpack();
        if Some(&code_hash) != CHEQUE_CODE_HASH.get() {
            return true;
        }
        match item {
            Item::Identity(ident) => {
                let ident = ident.parse();
                if ident.is_err() {
                    return true;
                }
                let (flag, auth) = ident.unwrap();
                if IdentityFlag::Ckb != flag {
                    return true;
                }
                let secp_lock_hash = self.get_secp_lock_hash_by_pubkey_hash(auth);
                if secp_lock_hash.is_err() {
                    return true;
                }
                let secp_lock_hash = secp_lock_hash.unwrap();
                let cell_args: Vec<u8> = cheque_cell.cell_output.lock().args().unpack();
                if self.is_unlock(
                    EpochNumberWithFraction::from_full_value(cheque_cell.epoch_number)
                        .to_rational(),
                    tip_epoch_number,
                    self.cheque_timeout.clone(),
                ) {
                    true
                } else {
                    cell_args[0..20] == secp_lock_hash.0
                }
            }
            _ => true,
        }
    }

    pub(crate) fn filter_cheque_record(
        &self,
        record: &Record,
        item: &Item,
        cell: &DetailedCell,
        tip_epoch_number: Option<RationalU256>,
    ) -> bool {
        let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if Some(&code_hash) != CHEQUE_CODE_HASH.get() {
            return true;
        }
        match item {
            Item::Identity(ident) => {
                let ident = ident.parse();
                if ident.is_err() {
                    return true;
                }
                let (flag, auth) = ident.unwrap();
                if IdentityFlag::Ckb != flag {
                    return true;
                }
                let secp_lock_hash = self.get_secp_lock_hash_by_pubkey_hash(auth);
                if secp_lock_hash.is_err() {
                    return true;
                }

                let secp_lock_hash = secp_lock_hash.unwrap();
                let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();

                // receiver
                if cell_args[0..20] == secp_lock_hash.0 {
                    return record.asset_info.asset_type != AssetType::CKB;
                }

                // sender capacity
                if record.asset_info.asset_type == AssetType::CKB {
                    return true;
                }

                // sender udt
                self.is_unlock(
                    EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational(),
                    tip_epoch_number,
                    self.cheque_timeout.clone(),
                )
            }
            _ => true,
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
        fee: Option<u64>,
    ) -> InnerResult<()> {
        // check inputs dup
        if has_duplication(
            transfer_components
                .inputs
                .iter()
                .map(|cell| &cell.out_point),
        ) {
            return Err(CoreError::InvalidTxPrebuilt("duplicate inputs".to_string()).into());
        }

        let required_capacity = calculate_required_capacity(
            &transfer_components.inputs,
            &transfer_components.outputs,
            fee,
            transfer_components.dao_reward_capacity,
        );

        let (required_capacity, _) = self
            .take_capacity_from_outputs(
                required_capacity,
                &mut transfer_components.outputs,
                &transfer_components.outputs_data,
                &from_items,
            )
            .await?;

        let mut header_dep_map: HashMap<packed::Byte32, usize> = HashMap::new();

        // when required_ckb > 0
        // balance capacity based on database
        // add new inputs
        let mut ckb_cells_cache = CkbCellsCache::new(from_items.to_owned());
        ckb_cells_cache
            .pagination
            .set_limit(Some(self.pool_cache_size));

        let required_capacity = self
            .pool_inputs_for_capacity(
                &ctx,
                &mut ckb_cells_cache,
                required_capacity,
                transfer_components,
                &mut header_dep_map,
            )
            .await?;

        if required_capacity.is_zero() {
            if fee.is_none() {
                return Ok(());
            }
            if transfer_components.fee_change_cell_index.is_some() {
                return Err(CoreError::InvalidTxPrebuilt("duplicate pool fee".to_string()).into());
            }

            if let Some(index) = self
                .find_acp_or_secp_belong_to_items(&transfer_components.outputs, &from_items)
                .await
            {
                transfer_components.fee_change_cell_index = Some(index);
                return Ok(());
            }
        } else if required_capacity.is_positive() {
            return Err(CoreError::InvalidTxPrebuilt("balance fail".to_string()).into());
        }

        // change
        let change_capacity =
            u64::try_from(required_capacity.unsigned_abs()).expect("impossible: overflow");

        let (change_capacity, change_address) = if let Ok(address) = self
            .get_a_change_address(
                change_capacity,
                &from_items,
                ckb_cells_cache.get_current_item_index(),
            )
            .await
        {
            (change_capacity, address)
        } else {
            if let Some(fee_index) = self
                .use_existed_cell_for_change(
                    &ctx,
                    change_capacity,
                    &from_items,
                    transfer_components,
                    &mut header_dep_map,
                )
                .await?
            {
                if fee.is_some() {
                    transfer_components.fee_change_cell_index = Some(fee_index);
                }
                return Ok(());
            }
            self.prepare_capacity_for_new_change_cell(
                &ctx,
                &mut ckb_cells_cache,
                change_capacity,
                transfer_components,
                &mut header_dep_map,
            )
            .await?
        };

        build_cell_for_output(
            change_capacity,
            change_address.payload().into(),
            None,
            None,
            &mut transfer_components.outputs,
            &mut transfer_components.outputs_data,
        )?;
        if fee.is_some() {
            transfer_components.fee_change_cell_index = Some(transfer_components.outputs.len() - 1);
        }

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn balance_transfer_tx_capacity_fee_by_output(
        &self,
        to_items: Vec<Item>,
        transfer_components: &mut TransferComponents,
        fee: u64,
    ) -> InnerResult<()> {
        if fee.is_zero() {
            return Err(CoreError::InvalidTxPrebuilt("fee is zero".to_string()).into());
        }
        let (required_capacity, index) = self
            .take_capacity_from_outputs(
                fee as i128,
                &mut transfer_components.outputs,
                &transfer_components.outputs_data,
                &to_items,
            )
            .await?;

        if required_capacity.is_zero() {
            if transfer_components.fee_change_cell_index.is_some() {
                return Err(CoreError::InvalidTxPrebuilt("duplicate pool fee".to_string()).into());
            }
            if let Some(index) = index {
                transfer_components.fee_change_cell_index = Some(index);
                return Ok(());
            }
        } else {
            return Err(CoreError::InvalidTxPrebuilt("failed to pay fee by to".to_string()).into());
        }
        Ok(())
    }

    async fn prepare_capacity_for_new_change_cell(
        &self,
        ctx: &Context,
        ckb_cells_cache: &mut CkbCellsCache,
        mut excessed_capacity: u64,
        transfer_components: &mut TransferComponents,
        header_dep_map: &mut HashMap<packed::Byte32, usize>,
    ) -> InnerResult<(u64, Address)> {
        // init
        let lock = if ckb_cells_cache.get_current_item_index() < ckb_cells_cache.items.len() {
            let item = &ckb_cells_cache.items[ckb_cells_cache.get_current_item_index()];
            self.get_default_owner_lock_by_item(item).await?
        } else {
            return Err(CoreError::CannotFindChangeCell.into());
        };
        let total_required = calculate_cell_capacity(
            &lock,
            &packed::ScriptOpt::default(),
            Capacity::bytes(0).expect("generate capacity"),
        );

        while excessed_capacity < total_required {
            let required_capacity = total_required - excessed_capacity;
            let (live_cell, asset_script_type) = self
                .pool_next_live_cell_for_capacity(
                    ctx.clone(),
                    ckb_cells_cache,
                    i128::from(required_capacity),
                    &transfer_components.inputs,
                )
                .await?;
            let capacity_provided = self
                .add_live_cell_for_balance_capacity(
                    ctx.clone(),
                    live_cell,
                    asset_script_type,
                    i128::from(required_capacity),
                    transfer_components,
                    header_dep_map,
                )
                .await;
            excessed_capacity += u64::try_from(capacity_provided).expect("impossible: overflow");
        }

        Ok((excessed_capacity, self.script_to_address(&lock)))
    }

    async fn use_existed_cell_for_change(
        &self,
        ctx: &Context,
        change_capacity: u64,
        from_items: &[Item],
        transfer_components: &mut TransferComponents,
        header_dep_map: &mut HashMap<packed::Byte32, usize>,
    ) -> InnerResult<Option<usize>> {
        // change tx outputs secp cell and acp cell belong to from
        if let Some(index) = self
            .find_acp_or_secp_belong_to_items(&transfer_components.outputs, from_items)
            .await
        {
            change_to_existed_cell(&mut transfer_components.outputs[index], change_capacity);
            return Ok(Some(index));
        }

        // change acp cell from db
        let mut cells_cache = AcpCellsCache::new(from_items.to_owned(), None);
        cells_cache.pagination.set_limit(Some(self.pool_cache_size));
        let ret = self
            .pool_next_live_acp_cell(ctx.clone(), &mut cells_cache, &transfer_components.inputs)
            .await;
        if let Ok((acp_cell, asset_script_type)) = ret {
            self.add_live_cell_for_balance_capacity(
                ctx.clone(),
                acp_cell,
                asset_script_type,
                -i128::from(change_capacity),
                transfer_components,
                header_dep_map,
            )
            .await;
            return Ok(Some(transfer_components.outputs.len() - 1));
        }

        Ok(None)
    }

    async fn find_acp_or_secp_belong_to_items(
        &self,
        cells: &[packed::CellOutput],
        items: &[Item],
    ) -> Option<usize> {
        for (index, output_cell) in cells.iter().enumerate().rev() {
            if self
                .is_acp_or_secp_belong_to_items(output_cell, items)
                .await
            {
                return Some(index);
            }
        }

        None
    }

    async fn take_capacity_from_outputs(
        &self,
        mut required_capacity: i128,
        outputs: &mut [packed::CellOutput],
        outputs_data: &[packed::Bytes],
        from_items: &[Item],
    ) -> InnerResult<(i128, Option<usize>)> {
        // when required_ckb > 0
        // balance capacity based on current tx
        // check outputs secp and acp belong to from
        let mut last_index = None;
        for (index, output_cell) in outputs.iter_mut().enumerate() {
            if required_capacity <= 0 {
                break;
            }

            if let Some((current_cell_capacity, cell_max_extra_capacity)) = self
                .caculate_current_and_extra_capacity(
                    output_cell,
                    outputs_data[index].clone(),
                    from_items,
                )
                .await
            {
                let took_capacity = if required_capacity >= cell_max_extra_capacity as i128 {
                    cell_max_extra_capacity
                } else {
                    u64::try_from(required_capacity).map_err(|_| CoreError::Overflow)?
                };

                let new_output_cell = output_cell
                    .clone()
                    .as_builder()
                    .capacity((current_cell_capacity - took_capacity).pack())
                    .build();
                *output_cell = new_output_cell;
                required_capacity -= took_capacity as i128;
                last_index = Some(index);
            }
        }

        Ok((required_capacity, last_index))
    }

    async fn pool_inputs_for_capacity(
        &self,
        ctx: &Context,
        ckb_cells_cache: &mut CkbCellsCache,
        mut required_capacity: i128,
        transfer_components: &mut TransferComponents,
        header_dep_map: &mut HashMap<packed::Byte32, usize>,
    ) -> InnerResult<i128> {
        loop {
            if required_capacity <= 0 {
                break;
            }
            let (live_cell, asset_script_type) = self
                .pool_next_live_cell_for_capacity(
                    ctx.clone(),
                    ckb_cells_cache,
                    required_capacity,
                    &transfer_components.inputs,
                )
                .await?;
            let capacity_provided = self
                .add_live_cell_for_balance_capacity(
                    ctx.clone(),
                    live_cell,
                    asset_script_type,
                    required_capacity,
                    transfer_components,
                    header_dep_map,
                )
                .await;
            required_capacity -= capacity_provided as i128;
        }

        Ok(required_capacity)
    }

    #[tracing_async]
    pub(crate) async fn balance_transfer_tx_udt(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        asset_info: AssetInfo,
        transfer_components: &mut TransferComponents,
    ) -> InnerResult<()> {
        // check inputs dup
        if has_duplication(
            transfer_components
                .inputs
                .iter()
                .map(|cell| &cell.out_point),
        ) {
            return Err(CoreError::InvalidTxPrebuilt("duplicate inputs".to_string()).into());
        }

        // check current balance
        let inputs_udt_amount = transfer_components
            .inputs
            .iter()
            .map::<u128, _>(|cell| decode_udt_amount(&cell.cell_data).unwrap_or(0))
            .sum::<u128>();
        let outputs_udt_amount = transfer_components
            .outputs_data
            .iter()
            .map::<u128, _>(|data| {
                let data: Bytes = data.unpack();
                decode_udt_amount(&data).unwrap_or(0)
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
        let mut udt_cells_cache = UdtCellsCache::new(from_items, asset_info.clone());
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
                    &transfer_components.inputs,
                )
                .await?;
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
        // only when receiver identity item claim udt in cheque cell
        if required_udt_amount < zero {
            let last_input_cell = transfer_components
                .inputs
                .last()
                .expect("impossible: get last input fail");
            let receiver_address = self
                .get_cheque_receiver_address(ctx.clone(), last_input_cell)
                .await?
                .to_string();

            // find acp
            if required_udt_amount < zero {
                let mut cells_cache = AcpCellsCache::new(
                    vec![Item::Identity(self.address_to_identity(&receiver_address)?)],
                    Some(asset_info.clone()),
                );
                cells_cache.pagination.set_limit(Some(self.pool_cache_size));
                let ret = self
                    .pool_next_live_acp_cell(
                        ctx.clone(),
                        &mut cells_cache,
                        &transfer_components.inputs,
                    )
                    .await;
                if let Ok((acp_cell, asset_script_type)) = ret {
                    let udt_amount_provided = self
                        .add_live_cell_for_balance_udt(
                            ctx.clone(),
                            acp_cell,
                            asset_script_type,
                            required_udt_amount.clone(),
                            transfer_components,
                        )
                        .await?;
                    required_udt_amount -= udt_amount_provided;
                }
            }

            // new output secp udt cell
            if required_udt_amount < zero {
                let change_udt_amount = required_udt_amount
                    .to_i128()
                    .expect("impossible: to i128 fail")
                    .unsigned_abs();
                let type_script = self
                    .build_sudt_type_script(ctx.clone(), blake2b_256_to_160(&asset_info.udt_hash))
                    .await?;
                let secp_address = self
                    .get_secp_address_by_item(&Item::Address(receiver_address))
                    .await?;
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

        Ok(())
    }

    pub async fn pool_next_live_cell_for_capacity(
        &self,
        ctx: Context,
        ckb_cells_cache: &mut CkbCellsCache,
        required_capacity: i128,
        used_input: &[DetailedCell],
    ) -> InnerResult<(DetailedCell, AssetScriptType)> {
        loop {
            if let Some((cell, asset_script_type)) = ckb_cells_cache.cell_deque.pop_front() {
                if self.is_in_cache(&cell.out_point)
                    || used_input.iter().any(|i| i.out_point == cell.out_point)
                {
                    continue;
                }
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
                    let cells = if let Ok(from_address) =
                        self.get_default_owner_address_by_item(&from_item).await
                    {
                        if is_secp256k1(&from_address) {
                            self.get_live_cells_by_item(
                                ctx.clone(),
                                from_item.clone(),
                                asset_ckb_set.clone(),
                                None,
                                None,
                                SECP256K1_CODE_HASH.get(),
                                Some(ExtraType::Dao),
                                &mut ckb_cells_cache.pagination,
                            )
                            .await?
                        } else if is_pw_lock(&from_address) {
                            self.get_live_cells_by_item(
                                ctx.clone(),
                                from_item.clone(),
                                asset_ckb_set.clone(),
                                None,
                                None,
                                PW_LOCK_CODE_HASH.get(),
                                Some(ExtraType::Dao),
                                &mut ckb_cells_cache.pagination,
                            )
                            .await?
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

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
                PoolCkbCategory::CkbCellbase => {
                    let mut asset_ckb_set = HashSet::new();
                    asset_ckb_set.insert(AssetInfo::new_ckb());
                    let ckb_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            asset_ckb_set.clone(),
                            None,
                            None,
                            SECP256K1_CODE_HASH.get(),
                            None,
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
                PoolCkbCategory::CkbNormalSecp => {
                    // database query optimization: when priority Cellbase and NormalSecp are next to each other
                    // database queries can be combined
                }
                PoolCkbCategory::CkbSecpUdt => {
                    let secp_udt_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            HashSet::new(),
                            None,
                            None,
                            SECP256K1_CODE_HASH.get(),
                            None,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let secp_udt_cells = secp_udt_cells
                        .into_iter()
                        .filter(|cell| {
                            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                                let type_code_hash: H256 = type_script.code_hash().unpack();
                                Some(&type_code_hash) == SUDT_CODE_HASH.get()
                            } else {
                                false
                            }
                        })
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = secp_udt_cells;
                }
                PoolCkbCategory::CkbAcp => {
                    let acp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            HashSet::new(),
                            None,
                            None,
                            ACP_CODE_HASH.get(),
                            None,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let acp_cells = acp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = acp_cells;
                }
                PoolCkbCategory::PwLockEthereum => {
                    let pw_lock_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            ckb_cells_cache.items[item_index].clone(),
                            HashSet::new(),
                            None,
                            None,
                            PW_LOCK_CODE_HASH.get(),
                            None,
                            &mut ckb_cells_cache.pagination,
                        )
                        .await?;
                    let pw_lock_cells = pw_lock_cells
                        .into_iter()
                        .filter(|cell| {
                            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                                let type_code_hash: H256 = type_script.code_hash().unpack();
                                Some(&type_code_hash) != DAO_CODE_HASH.get()
                            } else {
                                true
                            }
                        })
                        .map(|cell| (cell, AssetScriptType::PwLock))
                        .collect::<VecDeque<_>>();
                    ckb_cells_cache.cell_deque = pw_lock_cells;
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
        used_inputs: &[DetailedCell],
    ) -> InnerResult<(DetailedCell, AssetScriptType)> {
        let mut asset_udt_set = HashSet::new();
        asset_udt_set.insert(udt_cells_cache.asset_info.clone());

        loop {
            if let Some((cell, asset_script_type)) = udt_cells_cache.cell_deque.pop_front() {
                if self.is_in_cache(&cell.out_point)
                    || used_inputs.iter().any(|i| i.out_point == cell.out_point)
                {
                    continue;
                }
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
                PoolUdtCategory::CkbCheque => {
                    let cheque_cells_unlock = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            CHEQUE_CODE_HASH.get(),
                            None,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?
                        .into_iter()
                        .filter(|cell| {
                            self.filter_cheque_cell(&udt_cells_cache.items[item_index], cell, None)
                        })
                        .collect::<VecDeque<_>>();
                    if !cheque_cells_unlock.is_empty() {
                        udt_cells_cache.cell_deque = cheque_cells_unlock
                            .into_iter()
                            .map(|cell| {
                                (
                                    cell,
                                    AssetScriptType::Cheque(
                                        udt_cells_cache.items[item_index].clone(),
                                    ),
                                )
                            })
                            .collect::<VecDeque<_>>();
                    }
                }
                PoolUdtCategory::CkbSecpUdt => {
                    let secp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            SECP256K1_CODE_HASH.get(),
                            None,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let secp_cells = secp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::Secp256k1))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = secp_cells;
                }
                PoolUdtCategory::CkbAcp => {
                    let acp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            ACP_CODE_HASH.get(),
                            None,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let acp_cells = acp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = acp_cells;
                }
                PoolUdtCategory::PwLockEthereum => {
                    let pw_lock_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            udt_cells_cache.items[item_index].clone(),
                            asset_udt_set.clone(),
                            None,
                            None,
                            PW_LOCK_CODE_HASH.get(),
                            None,
                            &mut udt_cells_cache.pagination,
                        )
                        .await?;
                    let pw_lock_cells = pw_lock_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::PwLock))
                        .collect::<VecDeque<_>>();
                    udt_cells_cache.cell_deque = pw_lock_cells;
                }
            }
            if udt_cells_cache.pagination.cursor.is_none() {
                udt_cells_cache.array_index += 1;
            }
        }
    }

    pub async fn pool_next_live_acp_cell(
        &self,
        ctx: Context,
        acp_cells_cache: &mut AcpCellsCache,
        used_inputs: &[DetailedCell],
    ) -> InnerResult<(DetailedCell, AssetScriptType)> {
        loop {
            if let Some((cell, asset_script_type)) = acp_cells_cache.cell_deque.pop_front() {
                if self.is_in_cache(&cell.out_point)
                    || used_inputs.iter().any(|i| i.out_point == cell.out_point)
                {
                    continue;
                }
                return Ok((cell, asset_script_type));
            }

            if acp_cells_cache.array_index >= acp_cells_cache.item_category_array.len() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let (item_index, category_index) =
                acp_cells_cache.item_category_array[acp_cells_cache.array_index];

            let asset_infos = if let Some(asset_info) = acp_cells_cache.asset_info.clone() {
                let mut asset_udt_set = HashSet::new();
                asset_udt_set.insert(asset_info);
                asset_udt_set
            } else {
                HashSet::new()
            };

            match category_index {
                PoolAcpCategory::CkbAcp => {
                    let acp_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            acp_cells_cache.items[item_index].clone(),
                            asset_infos,
                            None,
                            None,
                            ACP_CODE_HASH.get(),
                            None,
                            &mut acp_cells_cache.pagination,
                        )
                        .await?;
                    let acp_cells = acp_cells
                        .into_iter()
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    acp_cells_cache.cell_deque = acp_cells;
                }
                PoolAcpCategory::PwLockEthereum => {
                    let pw_lock_cells = self
                        .get_live_cells_by_item(
                            ctx.clone(),
                            acp_cells_cache.items[item_index].clone(),
                            asset_infos,
                            None,
                            None,
                            PW_LOCK_CODE_HASH.get(),
                            None,
                            &mut acp_cells_cache.pagination,
                        )
                        .await?;
                    let pw_lock_cells = pw_lock_cells
                        .into_iter()
                        .filter(|cell| {
                            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                                let type_code_hash: H256 = type_script.code_hash().unpack();
                                Some(&type_code_hash) != DAO_CODE_HASH.get()
                            } else {
                                true
                            }
                        })
                        .map(|cell| (cell, AssetScriptType::ACP))
                        .collect::<VecDeque<_>>();
                    acp_cells_cache.cell_deque = pw_lock_cells;
                }
            }
            if acp_cells_cache.pagination.cursor.is_none() {
                acp_cells_cache.array_index += 1;
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
        header_dep_map: &mut HashMap<packed::Byte32, usize>,
    ) -> i128 {
        let provided_capacity = match asset_script_type {
            AssetScriptType::Secp256k1 => {
                if cell.cell_output.type_().is_none() {
                    transfer_components
                        .script_deps
                        .insert(SECP256K1.to_string());
                    let provided_capacity: u64 = cell.cell_output.capacity().unpack();
                    provided_capacity as i128
                } else {
                    let current_udt_amount = decode_udt_amount(&cell.cell_data).unwrap_or(0);
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
                }
            }
            AssetScriptType::ACP => {
                let current_capacity: u64 = cell.cell_output.capacity().unpack();
                let current_udt_amount = decode_udt_amount(&cell.cell_data);

                let provided_capacity = if cell.cell_output.type_().to_opt().is_some() {
                    transfer_components.script_deps.insert(SUDT.to_string());

                    let data_occupied = Capacity::bytes(cell.cell_data.len())
                        .expect("impossible: get data occupied capacity fail");
                    let occupied = cell
                        .cell_output
                        .occupied_capacity(data_occupied)
                        .expect("impossible: get cell occupied capacity fail")
                        .as_u64();

                    let max_provided_capacity = current_capacity.saturating_sub(occupied);
                    if required_capacity >= max_provided_capacity as i128 {
                        max_provided_capacity as i128
                    } else {
                        required_capacity
                    }
                } else {
                    // acp cell without type script will no longer keep
                    current_capacity as i128
                };

                if provided_capacity.is_zero() {
                    return provided_capacity;
                }

                transfer_components.script_deps.insert(ACP.to_string());

                if cell.cell_output.type_().to_opt().is_some() {
                    let outputs_capacity =
                        u64::try_from(current_capacity as i128 - provided_capacity)
                            .expect("impossible: overflow");
                    build_cell_for_output(
                        outputs_capacity,
                        cell.cell_output.lock(),
                        cell.cell_output.type_().to_opt(),
                        current_udt_amount,
                        &mut transfer_components.outputs,
                        &mut transfer_components.outputs_data,
                    )
                    .expect("impossible: build output cell fail");
                }
                provided_capacity
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
                    value: unlock_epoch.into(),
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

                let default_address = if let Ok(default_address) =
                    self.get_default_owner_address_by_item(&from_item).await
                {
                    default_address
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
                if !header_dep_map.contains_key(&deposit_block_hash) {
                    header_dep_map.insert(
                        deposit_block_hash.clone(),
                        transfer_components.header_deps.len(),
                    );
                    transfer_components
                        .header_deps
                        .push(deposit_block_hash.clone());
                }
                if !header_dep_map.contains_key(&withdrawing_block_hash) {
                    header_dep_map.insert(
                        withdrawing_block_hash.clone(),
                        transfer_components.header_deps.len(),
                    );
                    transfer_components.header_deps.push(withdrawing_block_hash);
                }

                // fill type_witness_args
                let deposit_block_hash_index_in_header_deps = header_dep_map
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

                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(DAO.to_string());
                if is_pw_lock(&default_address) {
                    transfer_components.script_deps.insert(PW_LOCK.to_string());
                }

                maximum_withdraw_capacity as i128
            }
            AssetScriptType::PwLock => {
                let current_capacity: u64 = cell.cell_output.capacity().unpack();
                let current_udt_amount = decode_udt_amount(&cell.cell_data);

                let provided_capacity = if cell.cell_output.type_().to_opt().is_some() {
                    transfer_components.script_deps.insert(SUDT.to_string());

                    let data_occupied = Capacity::bytes(cell.cell_data.len())
                        .expect("impossible: get data occupied capacity fail");
                    let occupied = cell
                        .cell_output
                        .occupied_capacity(data_occupied)
                        .expect("impossible: get cell occupied capacity fail")
                        .as_u64();

                    let max_provided_capacity = current_capacity.saturating_sub(occupied);
                    if required_capacity >= max_provided_capacity as i128 {
                        max_provided_capacity as i128
                    } else {
                        required_capacity
                    }
                } else {
                    // pw lock cell without type script will no longer keep
                    current_capacity as i128
                };

                if provided_capacity.is_zero() {
                    return provided_capacity;
                }

                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(PW_LOCK.to_string());

                if cell.cell_output.type_().to_opt().is_some() {
                    let outputs_capacity =
                        u64::try_from(current_capacity as i128 - provided_capacity)
                            .expect("impossible: overflow");
                    build_cell_for_output(
                        outputs_capacity,
                        cell.cell_output.lock(),
                        cell.cell_output.type_().to_opt(),
                        current_udt_amount,
                        &mut transfer_components.outputs,
                        &mut transfer_components.outputs_data,
                    )
                    .expect("impossible: build output cell fail");
                }

                provided_capacity
            }
            _ => unreachable!(),
        };

        transfer_components.inputs.push(cell);

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

        let provided_udt_amount = match asset_script_type {
            AssetScriptType::Cheque(item) => {
                transfer_components.script_deps.insert(CHEQUE.to_string());

                let sender_address = self.get_cheque_sender_address(ctx.clone(), &cell).await?;
                let sender_lock = address_to_script(sender_address.payload());
                let mut is_identity_receiver = false;
                if let Ok(ownership) = self.get_default_owner_address_by_item(&item).await {
                    if ownership != sender_address {
                        is_identity_receiver = true;
                    }
                }

                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data).unwrap_or(0);
                if required_udt_amount >= BigInt::from(max_provided_udt_amount)
                    || is_identity_receiver
                {
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
                }
            }
            AssetScriptType::Secp256k1 => {
                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());

                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data).unwrap_or(0);
                if required_udt_amount >= BigInt::from(max_provided_udt_amount) {
                    // convert to secp cell without type
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
                }
            }
            AssetScriptType::ACP => {
                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data).unwrap_or(0);
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

                provided_udt_amount
            }
            AssetScriptType::PwLock => {
                let max_provided_udt_amount = decode_udt_amount(&cell.cell_data).unwrap_or(0);
                let provided_udt_amount =
                    if required_udt_amount >= BigInt::from(max_provided_udt_amount) {
                        BigInt::from(max_provided_udt_amount)
                    } else {
                        required_udt_amount
                    };

                if provided_udt_amount.is_zero() {
                    return Ok(provided_udt_amount);
                }

                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(PW_LOCK.to_string());
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

                provided_udt_amount
            }
            _ => unreachable!(),
        };

        transfer_components.inputs.push(cell);

        Ok(provided_udt_amount)
    }

    pub async fn caculate_current_and_extra_capacity(
        &self,
        cell: &packed::CellOutput,
        cell_data: packed::Bytes,
        items: &[Item],
    ) -> Option<(u64, u64)> {
        if !self.is_acp_or_secp_belong_to_items(cell, items).await {
            return None;
        }

        let address = self.script_to_address(&cell.lock()).to_string();
        let address = Address::from_str(&address).map_err(CoreError::ParseAddressError);
        if let Ok(address) = address {
            if is_secp256k1(&address) {
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
            } else if is_acp(&address) | is_pw_lock(&address) {
                let current_capacity: u64 = cell.capacity().unpack();

                let cell_data: Bytes = cell_data.unpack();
                let data_occupied = Capacity::bytes(cell_data.len())
                    .expect("impossible: get data occupied capacity fail");
                let occupied = cell
                    .occupied_capacity(data_occupied)
                    .expect("impossible: get cell occupied capacity fail")
                    .as_u64();

                let extra_capacity = current_capacity.saturating_sub(occupied);
                Some((current_capacity, extra_capacity))
            } else {
                None
            }
        } else {
            None
        }
    }

    async fn is_acp_or_secp_belong_to_items(
        &self,
        cell: &packed::CellOutput,
        items: &[Item],
    ) -> bool {
        let cell_address = self.script_to_address(&cell.lock()).to_string();
        let item_of_cell = if let Ok(identity) = self.address_to_identity(&cell_address) {
            Item::Identity(identity)
        } else {
            return false;
        };
        let default_address_of_cell =
            if let Ok(address) = self.get_default_owner_address_by_item(&item_of_cell).await {
                address
            } else {
                return false;
            };
        if let Some(type_script) = cell.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();
            if Some(&type_code_hash) != SUDT_CODE_HASH.get() {
                return false;
            }
        }
        for item in items {
            let ret = self.get_default_owner_address_by_item(item).await;
            if let Ok(default_address_of_item) = ret {
                if default_address_of_item == default_address_of_cell {
                    return true;
                }
            } else {
                continue;
            }
        }
        false
    }

    pub(crate) async fn is_items_contain_addresses(
        &self,
        items: &[JsonItem],
        addresses: &[String],
    ) -> InnerResult<bool> {
        let mut from_ownership_lock_hash_set = HashSet::new();
        for json_item in items {
            let item = Item::try_from(json_item.to_owned())?;
            let lock_hash = self.get_default_owner_lock_by_item(&item).await;
            if let Ok(lock_hash) = lock_hash {
                from_ownership_lock_hash_set.insert(lock_hash);
            }
        }
        for to_address in addresses {
            if let Ok(identity) = self.address_to_identity(to_address) {
                let to_item = Item::Identity(identity);
                let to_ownership_lock_hash = self.get_default_owner_lock_by_item(&to_item).await?;
                if from_ownership_lock_hash_set.contains(&to_ownership_lock_hash) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn get_builtin_script(&self, builtin_script_name: &str, args: H160) -> packed::Script {
        self.builtin_scripts
            .get(builtin_script_name)
            .cloned()
            .expect("Impossible: get built in script fail")
            .script
            .as_builder()
            .args(args.0.pack())
            .build()
    }

    fn get_type_hashes(
        &self,
        asset_infos: HashSet<AssetInfo>,
        extra: Option<ExtraType>,
    ) -> Vec<H256> {
        let dao_script_hash: H256 = self
            .builtin_scripts
            .get(DAO)
            .cloned()
            .expect("get dao script info")
            .script
            .calc_script_hash()
            .unpack();
        if asset_infos.is_empty() {
            if extra == Some(ExtraType::Dao) {
                vec![dao_script_hash]
            } else {
                vec![]
            }
        } else {
            asset_infos
                .into_iter()
                .filter(|asset_info| {
                    !(extra == Some(ExtraType::Dao) && asset_info.asset_type == AssetType::UDT)
                })
                .map(|asset_info| match asset_info.asset_type {
                    AssetType::CKB => match extra {
                        Some(ExtraType::Dao) => dao_script_hash.clone(),
                        _ => H256::default(),
                    },
                    AssetType::UDT => asset_info.udt_hash,
                })
                .collect()
        }
    }

    pub fn address_to_identity(&self, address: &str) -> InnerResult<Identity> {
        let address = Address::from_str(address).map_err(CoreError::ParseAddressError)?;
        let script = address_to_script(address.payload());
        let pub_key_hash = script.args().as_slice()[4..24].to_vec();

        if is_secp256k1(&address) || is_acp(&address) {
            return Ok(Identity::new(
                IdentityFlag::Ckb,
                H160::from_slice(&pub_key_hash).expect("get pubkey hash h160"),
            ));
        };

        if is_pw_lock(&address) {
            return Ok(Identity::new(
                IdentityFlag::Ethereum,
                H160::from_slice(&pub_key_hash).expect("get pubkey hash h160"),
            ));
        }

        Err(CoreError::UnsupportLockScript(hex::encode(script.code_hash().as_slice())).into())
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

pub fn to_since(config: SinceConfig) -> InnerResult<u64> {
    let since = match (config.flag, config.type_) {
        (SinceFlag::Absolute, SinceType::BlockNumber) => 0b0000_0000u64,
        (SinceFlag::Relative, SinceType::BlockNumber) => 0b1000_0000u64,
        (SinceFlag::Absolute, SinceType::EpochNumber) => 0b0010_0000u64,
        (SinceFlag::Relative, SinceType::EpochNumber) => 0b1010_0000u64,
        (SinceFlag::Absolute, SinceType::Timestamp) => 0b0100_0000u64,
        (SinceFlag::Relative, SinceType::Timestamp) => 0b1100_0000u64,
    };
    let value: u64 = config.value.into();
    if value > 0xff_ffff_ffff_ffffu64 {
        return Err(CoreError::InvalidRpcParams(
            "the value in the since config is too large".to_string(),
        )
        .into());
    }
    Ok((since << 56) + value)
}

pub fn build_cheque_args(receiver_address: Address, sender_address: Address) -> packed::Bytes {
    let mut ret = blake2b_160(address_to_script(receiver_address.payload()).as_slice()).to_vec();
    let sender = blake2b_160(address_to_script(sender_address.payload()).as_slice());
    ret.extend_from_slice(&sender);
    ret.pack()
}

pub(crate) fn dedup_json_items(items: &mut Vec<JsonItem>) {
    let mut set = HashSet::new();
    items.retain(|i| set.insert(i.clone()));
}

pub(crate) fn calculate_the_percentage(numerator: u64, denominator: u64) -> String {
    if denominator.is_zero() {
        "0.00000%".to_string()
    } else {
        let percentage = numerator as f64 / denominator as f64;
        format!("{:.5}%", 100.0 * percentage)
    }
}

fn has_duplication<T: std::hash::Hash + std::cmp::Eq, I: ExactSizeIterator + Iterator<Item = T>>(
    iter: I,
) -> bool {
    let origin_len = iter.len();
    let set: HashSet<T> = iter.collect();

    origin_len != set.len()
}

fn calculate_required_capacity(
    inputs: &[DetailedCell],
    outputs: &[packed::CellOutput],
    fee: Option<u64>,
    dao_reward: u64,
) -> i128 {
    let inputs_capacity = inputs
        .iter()
        .map::<u64, _>(|cell| cell.cell_output.capacity().unpack())
        .sum::<u64>();
    let outputs_capacity = outputs
        .iter()
        .map::<u64, _>(|cell| cell.capacity().unpack())
        .sum::<u64>();
    let fee = if let Some(fee) = fee { fee } else { 0 };

    (outputs_capacity + fee) as i128 - (inputs_capacity + dao_reward) as i128
}

fn change_to_existed_cell(output: &mut packed::CellOutput, change_capacity: u64) {
    let current_capacity: u64 = output.capacity().unpack();
    let new_capacity = current_capacity + change_capacity;
    let new_output_cell = output
        .clone()
        .as_builder()
        .capacity(new_capacity.pack())
        .build();
    *output = new_output_cell;
}

pub(crate) fn calculate_cell_capacity(
    lock: &packed::Script,
    type_: &packed::ScriptOpt,
    data_capacity: Capacity,
) -> u64 {
    Capacity::bytes(8)
        .and_then(|x| x.safe_add(data_capacity))
        .and_then(|x| lock.occupied_capacity().and_then(|y| y.safe_add(x)))
        .and_then(|x| {
            type_
                .to_opt()
                .as_ref()
                .map(packed::Script::occupied_capacity)
                .transpose()
                .and_then(|y| y.unwrap_or_else(Capacity::zero).safe_add(x))
        })
        .expect("calculate_cell_capacity")
        .as_u64()
}

pub(crate) fn map_json_items(json_items: Vec<JsonItem>) -> InnerResult<Vec<Item>> {
    let items = json_items
        .into_iter()
        .map(Item::try_from)
        .collect::<Result<Vec<Item>, _>>()?;
    Ok(items)
}
