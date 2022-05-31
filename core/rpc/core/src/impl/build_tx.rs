use crate::r#impl::utils::{
    build_cell_for_output, build_cheque_args, calculate_cell_capacity,
    calculate_unlock_epoch_number, dedup_json_items, is_dao_withdraw_unlock, to_since,
};
use crate::r#impl::{address_to_script, utils_types, utils_types::TransferComponents};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{
    Capacity, EpochNumberWithFraction, ScriptHashType, TransactionBuilder, TransactionView,
};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256, U256};
use common::address::{is_acp, is_pw_lock, is_secp256k1};
use common::hash::blake2b_256_to_160;
use common::lazy::{
    ACP_CODE_HASH, DAO_CODE_HASH, PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH,
};
use common::utils::{decode_udt_amount, encode_udt_amount};
use common::{
    Address, Context, DetailedCell, PaginationRequest, ACP, CHEQUE, DAO, PW_LOCK, SECP256K1, SUDT,
};
use common_logger::tracing_async;
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{
    BYTE_SHANNONS, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM, MIN_DAO_CAPACITY,
};
use core_rpc_types::lazy::CURRENT_EPOCH_NUMBER;
use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload, ExtraType, Item,
    JsonItem, OutputCapacityProvider, PayFee, ScriptGroup, ScriptGroupType, SimpleTransferPayload,
    SinceConfig, SinceFlag, SinceType, SudtIssuePayload, ToInfo, TransactionCompletionResponse,
    TransferPayload,
};
use core_storage::Storage;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::convert::TryFrom;
use std::slice::Iter;
use std::str::FromStr;
use std::vec;

#[derive(Default, Clone, Debug)]
pub struct CellWithData {
    pub cell: packed::CellOutput,
    pub data: packed::Bytes,
}

impl<C: CkbRpc> MercuryRpcImpl<C> {
    #[tracing_async]
    pub(crate) async fn inner_build_dao_deposit_transaction(
        &self,
        ctx: Context,
        mut payload: DaoDepositPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.is_empty() {
            return Err(CoreError::NeedAtLeastOneFrom.into());
        }
        if payload.from.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        if MIN_DAO_CAPACITY > payload.amount.into() {
            return Err(CoreError::InvalidDAOCapacity.into());
        }
        dedup_json_items(&mut payload.from);
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_dao_deposit_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_deposit_transaction(
        &self,
        ctx: Context,
        payload: DaoDepositPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build the outputs
        let mut transfer_components = utils_types::TransferComponents::new();

        let items = map_json_items(payload.from)?;

        // build output deposit cell
        let deposit_address = match payload.to {
            Some(address) => Address::from_str(&address).map_err(CoreError::InvalidRpcParams)?,
            None => self.get_default_owner_address_by_item(&items[0]).await?,
        };
        let type_script = self
            .get_script_builder(DAO)?
            .hash_type(ScriptHashType::Type.into())
            .build();
        let output_deposit = packed::CellOutputBuilder::default()
            .capacity(payload.amount.pack())
            .lock(deposit_address.payload().into())
            .type_(Some(type_script).pack())
            .build();
        let output_data_deposit: packed::Bytes = Bytes::from(vec![0u8; 8]).pack();
        transfer_components.outputs.push(output_deposit);
        transfer_components.outputs_data.push(output_data_deposit);

        // build script_deps
        transfer_components.script_deps.insert(DAO.to_string());

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            items,
            vec![],
            None,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    pub(crate) async fn inner_build_dao_withdraw_transaction(
        &self,
        ctx: Context,
        mut payload: DaoWithdrawPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.is_empty() {
            return Err(CoreError::NeedAtLeastOneFrom.into());
        }
        if payload.from.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        dedup_json_items(&mut payload.from);
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_dao_withdraw_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_withdraw_transaction(
        &self,
        ctx: Context,
        payload: DaoWithdrawPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build the outputs
        let mut transfer_components = TransferComponents::new();

        let mut deposit_cells = vec![];
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        for from_item in payload.from.clone() {
            let from_item = Item::try_from(from_item)?;
            let address = self.get_default_owner_address_by_item(&from_item).await?;

            let lock_filter = if is_secp256k1(&address) {
                SECP256K1_CODE_HASH.get()
            } else if is_pw_lock(&address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                continue;
            };

            // get deposit cells
            let mut cells = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    from_item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    lock_filter,
                    Some(ExtraType::Dao),
                    &mut PaginationRequest::default(),
                )
                .await?;
            deposit_cells.append(&mut cells);
        }

        let mut set = HashSet::new();
        deposit_cells.retain(|i| set.insert(i.clone()));

        let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
        let deposit_cells = deposit_cells
            .into_iter()
            .filter(|cell| cell.cell_data == Box::new([0u8; 8]).to_vec())
            .filter(|cell| {
                (EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational()
                    + U256::from(4u64))
                    < tip_epoch_number
            })
            .collect::<Vec<_>>();
        if deposit_cells.is_empty() {
            return Err(CoreError::CannotFindDepositCell.into());
        }

        // build header_deps
        let mut header_deps = HashSet::new();
        for cell in &deposit_cells {
            header_deps.insert(cell.block_hash.pack());
        }
        transfer_components
            .header_deps
            .append(&mut header_deps.into_iter().collect());

        // build inputs
        transfer_components.inputs.extend_from_slice(&deposit_cells);

        // build output withdrawing cells
        let mut outputs_withdraw: Vec<packed::CellOutput> = deposit_cells
            .iter()
            .map(|cell| {
                let cell_output = &cell.cell_output;
                packed::CellOutputBuilder::default()
                    .capacity(cell_output.capacity())
                    .lock(cell_output.lock())
                    .type_(cell_output.type_())
                    .build()
            })
            .collect();
        let mut outputs_data_withdraw: Vec<packed::Bytes> = deposit_cells
            .iter()
            .map(|cell| {
                let data: packed::Uint64 = cell.block_number.pack();
                data.as_bytes().pack()
            })
            .collect();

        // build outputs
        transfer_components.outputs.append(&mut outputs_withdraw);
        transfer_components
            .outputs_data
            .append(&mut outputs_data_withdraw);

        // build script_deps
        transfer_components
            .script_deps
            .insert(SECP256K1.to_string());
        transfer_components.script_deps.insert(DAO.to_string());
        for cell in deposit_cells {
            if self.is_script(&cell.cell_output.lock(), PW_LOCK)? {
                transfer_components.script_deps.insert(PW_LOCK.to_string());
                break;
            }
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            vec![],
            None,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    pub(crate) async fn inner_build_dao_claim_transaction(
        &self,
        ctx: Context,
        mut payload: DaoClaimPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.is_empty() {
            return Err(CoreError::NeedAtLeastOneFrom.into());
        }
        if payload.from.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        dedup_json_items(&mut payload.from);
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_dao_claim_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_claim_transaction(
        &self,
        ctx: Context,
        payload: DaoClaimPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        let from_items = map_json_items(payload.from)?;

        let to_address = match payload.to {
            Some(address) => Address::from_str(&address).map_err(CoreError::InvalidRpcParams)?,
            None => {
                self.get_default_owner_address_by_item(&from_items[0])
                    .await?
            }
        };

        let mut withdrawing_cells = vec![];
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        for from_item in from_items {
            let from_address = self.get_default_owner_address_by_item(&from_item).await?;

            let lock_filter = if is_secp256k1(&from_address) {
                SECP256K1_CODE_HASH.get()
            } else if is_pw_lock(&from_address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                continue;
            };

            // get withdrawing cells including in lock period
            let mut cells = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    from_item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    lock_filter,
                    Some(ExtraType::Dao),
                    &mut PaginationRequest::default(),
                )
                .await?;
            withdrawing_cells.append(&mut cells);
        }

        let mut set = HashSet::new();
        withdrawing_cells.retain(|i| set.insert(i.clone()));

        let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
        let withdrawing_cells = withdrawing_cells
            .into_iter()
            .filter(|cell| {
                cell.cell_data != Box::new([0u8; 8]).to_vec() && cell.cell_data.len() == 8
            })
            .filter(|cell| {
                EpochNumberWithFraction::from_full_value(cell.epoch_number).to_rational()
                    + U256::from(4u64)
                    < tip_epoch_number
            })
            .collect::<Vec<_>>();
        if withdrawing_cells.is_empty() {
            return Err(CoreError::CannotFindUnlockedWithdrawingCell.into());
        }

        // init transfer components: build the outputs
        let mut transfer_components = TransferComponents::new();
        let mut header_dep_map: HashMap<packed::Byte32, usize> = HashMap::new();
        let mut maximum_withdraw_capacity = 0;

        for withdrawing_cell in withdrawing_cells {
            // get deposit_cell
            let withdrawing_tx = self
                .inner_get_transaction_with_status(
                    ctx.clone(),
                    withdrawing_cell.out_point.tx_hash().unpack(),
                )
                .await?;
            let withdrawing_tx_input_index: u32 = withdrawing_cell.out_point.index().unpack(); // input deposite cell has the same index
            let deposit_cell = &withdrawing_tx.input_cells[withdrawing_tx_input_index as usize];

            if !is_dao_withdraw_unlock(
                EpochNumberWithFraction::from_full_value(deposit_cell.epoch_number).to_rational(),
                EpochNumberWithFraction::from_full_value(withdrawing_cell.epoch_number)
                    .to_rational(),
                Some((**CURRENT_EPOCH_NUMBER.load()).clone()),
            ) {
                continue;
            }

            // calculate input since
            let unlock_epoch = calculate_unlock_epoch_number(
                deposit_cell.epoch_number,
                withdrawing_cell.epoch_number,
            );
            let since = to_since(SinceConfig {
                type_: SinceType::EpochNumber,
                flag: SinceFlag::Absolute,
                value: unlock_epoch.into(),
            })?;

            // build input
            transfer_components
                .dao_since_map
                .insert(transfer_components.inputs.len(), since);

            // build header deps
            let deposit_block_hash = deposit_cell.block_hash.pack();
            let withdrawing_block_hash = withdrawing_cell.block_hash.pack();
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

            // build script deps
            if self.is_script(&withdrawing_cell.cell_output.lock(), PW_LOCK)? {
                transfer_components.script_deps.insert(PW_LOCK.to_string());
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

            // calculate maximum_withdraw_capacity
            maximum_withdraw_capacity += self
                .calculate_maximum_withdraw(
                    ctx.clone(),
                    &withdrawing_cell,
                    deposit_cell.block_hash.clone(),
                    withdrawing_cell.block_hash.clone(),
                )
                .await?;

            transfer_components.inputs.push(withdrawing_cell);
        }

        if transfer_components.inputs.is_empty() {
            return Err(CoreError::CannotFindUnlockedWithdrawingCell.into());
        }

        // build output cell
        let output_cell_capacity = maximum_withdraw_capacity - fixed_fee;
        let change_cell_index = build_cell_for_output(
            output_cell_capacity,
            to_address.payload().into(),
            None,
            None,
            &mut transfer_components.outputs,
            &mut transfer_components.outputs_data,
        )?;

        // build script deps
        transfer_components
            .script_deps
            .insert(SECP256K1.to_string());
        transfer_components.script_deps.insert(DAO.to_string());

        self.complete_prebuild_transaction(transfer_components, None)
            .map(|(tx_view, script_groups)| (tx_view, script_groups, change_cell_index))
    }

    #[tracing_async]
    pub(crate) async fn inner_build_transfer_transaction(
        &self,
        ctx: Context,
        mut payload: TransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.is_empty() || payload.to.is_empty() {
            return Err(CoreError::NeedAtLeastOneFromAndOneTo.into());
        }
        if payload.from.len() > MAX_ITEM_NUM || payload.to.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        dedup_json_items(&mut payload.from);
        let addresses: Vec<String> = payload
            .to
            .iter()
            .map(|to_info| to_info.address.to_owned())
            .collect();
        if self
            .is_items_contain_addresses(&payload.from, &addresses)
            .await?
        {
            return Err(CoreError::FromContainTo.into());
        }
        for to_info in &payload.to {
            if 0u128 == to_info.amount.into() {
                return Err(CoreError::TransferAmountMustPositive.into());
            }
        }
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_transfer_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_transfer_transaction(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        match (
            &payload.asset_info.asset_type,
            &payload.output_capacity_provider,
        ) {
            (AssetType::CKB, Some(OutputCapacityProvider::From)) => {
                self.prebuild_ckb_transfer_transaction_from_provide_capacity(
                    ctx, payload, fixed_fee,
                )
                .await
            }
            (AssetType::CKB, None | Some(OutputCapacityProvider::To)) => {
                self.prebuild_ckb_transfer_transaction_to_provide_capacity(ctx, payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Some(OutputCapacityProvider::From)) => {
                self.prebuild_udt_transfer_transaction_from_provide_capacity(
                    ctx, payload, fixed_fee,
                )
                .await
            }
            (AssetType::UDT, None | Some(OutputCapacityProvider::To)) => {
                self.prebuild_udt_transfer_transaction_to_provide_capacity(ctx, payload, fixed_fee)
                    .await
            }
        }
    }

    #[tracing_async]
    async fn prebuild_ckb_transfer_transaction_from_provide_capacity(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build the outputs
        let mut transfer_components = utils_types::TransferComponents::new();

        for to in &payload.to {
            let to_capacity: u128 = to.amount.into();
            let to_capacity =
                u64::try_from(to_capacity).map_err(|_| CoreError::RequiredCKBMoreThanMax)?;
            let to_address = Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            let min_capacity = calculate_cell_capacity(
                &address_to_script(to_address.payload()),
                &packed::ScriptOpt::default(),
                Capacity::bytes(0).expect("generate capacity"),
            );
            if to_capacity < min_capacity {
                return Err(CoreError::RequiredCKBLessThanMin.into());
            }
            build_cell_for_output(
                to_capacity,
                to_address.payload().into(),
                None,
                None,
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            payload.to.into_iter().map(|info| info.address).collect(),
            payload.since,
            payload.pay_fee,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_ckb_transfer_transaction_to_provide_capacity(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = TransferComponents::new();

        for to in &payload.to {
            let to_address = Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            let live_acps: Vec<DetailedCell> = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    Item::Address(to.address.to_string()),
                    HashSet::new(),
                    None,
                    None,
                    None,
                    None,
                    &mut PaginationRequest::default(),
                )
                .await?
                .into_iter()
                .filter(|cell| {
                    if let Some(type_script) = cell.cell_output.type_().to_opt() {
                        let type_code_hash: H256 = type_script.code_hash().unpack();
                        type_code_hash != *DAO_CODE_HASH.get().expect("get dao code hash")
                    } else {
                        true
                    }
                })
                .collect();
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let live_acp = live_acps[0].clone();
            let current_capacity: u64 = live_acp.cell_output.capacity().unpack();
            let current_udt_amount = decode_udt_amount(&live_acp.cell_data);
            transfer_components.inputs.push(live_acp.clone());
            transfer_components
                .inputs_not_require_signature
                .insert(transfer_components.inputs.len() - 1);

            if is_acp(&to_address) {
                transfer_components.script_deps.insert(ACP.to_string());
            }
            if is_pw_lock(&to_address) {
                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(PW_LOCK.to_string());
            }
            if let Some(type_script) = live_acp.cell_output.type_().to_opt() {
                let type_code_hash: H256 = type_script.code_hash().unpack();
                if type_code_hash == *SUDT_CODE_HASH.get().expect("get sudt code hash") {
                    transfer_components.script_deps.insert(SUDT.to_string());
                }
            }

            // build acp output
            let required_capacity: u128 = to.amount.into();
            build_cell_for_output(
                current_capacity + required_capacity as u64,
                live_acp.cell_output.lock(),
                live_acp.cell_output.type_().to_opt(),
                current_udt_amount,
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            payload.to.into_iter().map(|info| info.address).collect(),
            payload.since,
            payload.pay_fee,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_udt_transfer_transaction_from_provide_capacity(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        for to in &payload.to {
            let to_address = Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            let to_lock = address_to_script(to_address.payload());
            let sudt_type_script = self
                .build_sudt_type_script(
                    ctx.clone(),
                    blake2b_256_to_160(&payload.asset_info.udt_hash),
                )
                .await?;
            let sudt_type_script = Some(sudt_type_script).pack();
            let udt_amount = to.amount.into();
            let capacity = calculate_cell_capacity(
                &to_lock,
                &sudt_type_script,
                Capacity::bytes(Bytes::from(encode_udt_amount(udt_amount)).len())
                    .expect("generate capacity"),
            );

            build_cell_for_output(
                capacity,
                to_lock,
                sudt_type_script.to_opt(),
                Some(udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
            transfer_components.script_deps.insert(SUDT.to_string());
        }

        // balance udt
        let from_items = payload
            .from
            .iter()
            .map(|json_item| Item::try_from(json_item.to_owned()))
            .collect::<Result<Vec<Item>, _>>()?;
        self.balance_transfer_tx_udt(
            ctx.clone(),
            from_items,
            payload.clone().asset_info,
            &mut transfer_components,
        )
        .await?;

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            payload.to.into_iter().map(|info| info.address).collect(),
            payload.since,
            payload.pay_fee,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_udt_transfer_transaction_to_provide_capacity(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        let mut asset_set = HashSet::new();
        asset_set.insert(payload.asset_info.clone());

        for to in &payload.to {
            let to_address =
                Address::from_str(&to.address).map_err(CoreError::ParseAddressError)?;
            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    Item::Address(to.address.to_string()),
                    asset_set.clone(),
                    None,
                    None,
                    None,
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?;
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let live_acp = live_acps[0].clone();
            let existing_udt_amount = decode_udt_amount(&live_acp.cell_data).unwrap_or(0);
            transfer_components.inputs.push(live_acp.clone());
            transfer_components
                .inputs_not_require_signature
                .insert(transfer_components.inputs.len() - 1);

            if is_acp(&to_address) {
                transfer_components.script_deps.insert(ACP.to_string());
            }
            if is_pw_lock(&to_address) {
                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(PW_LOCK.to_string());
            }
            transfer_components.script_deps.insert(SUDT.to_string());

            // build acp output
            let to_udt_amount: u128 = to.amount.into();
            build_cell_for_output(
                live_acp.cell_output.capacity().unpack(),
                live_acp.cell_output.lock(),
                live_acp.cell_output.type_().to_opt(),
                Some(existing_udt_amount + to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance udt
        let from_items = payload
            .from
            .iter()
            .map(|json_item| Item::try_from(json_item.to_owned()))
            .collect::<Result<Vec<Item>, _>>()?;
        self.balance_transfer_tx_udt(
            ctx.clone(),
            from_items,
            payload.clone().asset_info,
            &mut transfer_components,
        )
        .await?;

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            payload.to.into_iter().map(|info| info.address).collect(),
            payload.since,
            payload.pay_fee,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    pub(crate) async fn inner_build_simple_transfer_transaction(
        &self,
        ctx: Context,
        payload: SimpleTransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_simple_transfer_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    async fn prebuild_simple_transfer_transaction(
        &self,
        ctx: Context,
        payload: SimpleTransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        if payload.from.is_empty() || payload.to.is_empty() {
            return Err(CoreError::NeedAtLeastOneFromAndOneTo.into());
        }
        if payload.from.len() > MAX_ITEM_NUM || payload.to.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        let mut from_items = payload
            .from
            .iter()
            .map(|address| {
                self.address_to_identity(address)
                    .map(|identity| JsonItem::Identity(identity.encode()))
            })
            .collect::<Result<Vec<JsonItem>, _>>()?;
        dedup_json_items(&mut from_items);
        let addresses: Vec<String> = payload
            .to
            .iter()
            .map(|to_info| to_info.address.to_owned())
            .collect();
        if self
            .is_items_contain_addresses(&from_items, &addresses)
            .await?
        {
            return Err(CoreError::FromContainTo.into());
        }
        for to_info in &payload.to {
            if 0u128 == to_info.amount.into() {
                return Err(CoreError::TransferAmountMustPositive.into());
            }
        }
        let to_items = payload
            .to
            .iter()
            .map(|ToInfo { address, .. }| self.address_to_identity(address).map(Item::Identity))
            .collect::<Result<Vec<Item>, _>>()?;

        match payload.asset_info.asset_type {
            AssetType::CKB => {
                let transfer_payload = TransferPayload {
                    asset_info: payload.asset_info,
                    from: from_items,
                    to: payload.to,
                    output_capacity_provider: Some(OutputCapacityProvider::From),
                    pay_fee: None,
                    fee_rate: payload.fee_rate,
                    since: payload.since,
                };
                self.prebuild_ckb_transfer_transaction_from_provide_capacity(
                    ctx,
                    transfer_payload,
                    fixed_fee,
                )
                .await
            }

            AssetType::UDT => {
                let mut asset_infos = HashSet::new();
                asset_infos.insert(payload.asset_info.clone());
                let output_capacity_provider = self
                    .get_simple_transfer_output_capacity_provider(
                        ctx.clone(),
                        &to_items,
                        asset_infos,
                    )
                    .await?;
                let mut transfer_payload = TransferPayload {
                    asset_info: payload.asset_info,
                    from: from_items,
                    to: payload.to,
                    output_capacity_provider: Some(output_capacity_provider),
                    pay_fee: None,
                    fee_rate: payload.fee_rate,
                    since: payload.since,
                };
                match output_capacity_provider {
                    OutputCapacityProvider::From => {
                        let mut to_infos = vec![];
                        for to in &transfer_payload.to {
                            let receiver_address = Address::from_str(&to.address)
                                .map_err(CoreError::InvalidRpcParams)?;
                            if !is_secp256k1(&receiver_address) {
                                return Err(CoreError::InvalidRpcParams(
                                    "Every to address should be secp/256k1 address".to_string(),
                                )
                                .into());
                            }
                            let sender_address = {
                                let item = Item::Address(payload.from[0].to_owned());
                                self.get_secp_address_by_item(&item).await?
                            };
                            let cheque_args = build_cheque_args(receiver_address, sender_address);
                            let cheque_lock = self
                                .get_script_builder(CHEQUE)?
                                .args(cheque_args)
                                .hash_type(ScriptHashType::Type.into())
                                .build();
                            let cheque_address = self.script_to_address(&cheque_lock);
                            to_infos.push(ToInfo {
                                address: cheque_address.to_string(),
                                amount: to.amount,
                            });
                        }
                        transfer_payload.to = to_infos;
                        self.prebuild_udt_transfer_transaction_from_provide_capacity(
                            ctx,
                            transfer_payload,
                            fixed_fee,
                        )
                        .await
                    }
                    OutputCapacityProvider::To => {
                        let mut to_infos = vec![];
                        for to in &transfer_payload.to {
                            let acp_address = self
                                .get_acp_address_by_item(&Item::Address(to.address.clone()))
                                .await?;
                            to_infos.push(ToInfo {
                                address: acp_address.to_string(),
                                amount: to.amount,
                            });
                        }
                        transfer_payload.to = to_infos;
                        self.prebuild_udt_transfer_transaction_to_provide_capacity(
                            ctx,
                            transfer_payload,
                            fixed_fee,
                        )
                        .await
                    }
                }
            }
        }
    }

    #[tracing_async]
    pub(crate) async fn build_transaction_with_adjusted_fee<'a, F, Fut, T>(
        &'a self,
        prebuild: F,
        ctx: Context,
        payload: T,
        fee_rate: Option<u64>,
    ) -> InnerResult<TransactionCompletionResponse>
    where
        F: Fn(&'a MercuryRpcImpl<C>, Context, T, u64) -> Fut + Copy,
        Fut: std::future::Future<Output = InnerResult<(TransactionView, Vec<ScriptGroup>, usize)>>,
        T: Clone,
    {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let (tx_view, script_groups, change_cell_index) =
                prebuild(self, ctx.clone(), payload.clone(), estimate_fee).await?;
            let tx_size = calculate_tx_size(&tx_view);
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }

            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let tx_view = self.update_tx_view_change_cell_by_index(
                    tx_view.into(),
                    change_cell_index,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response = TransactionCompletionResponse::new(tx_view, script_groups);
                return Ok(adjust_response);
            }
        }
    }

    #[tracing_async]
    async fn get_simple_transfer_output_capacity_provider(
        &self,
        ctx: Context,
        to_items: &[Item],
        asset_infos: HashSet<AssetInfo>,
    ) -> InnerResult<OutputCapacityProvider> {
        for i in to_items {
            let to_address = self.get_default_owner_address_by_item(i).await?;
            let lock_filter = if is_secp256k1(&to_address) {
                ACP_CODE_HASH.get()
            } else if is_pw_lock(&to_address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                return Ok(OutputCapacityProvider::From);
            };

            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    i.to_owned(),
                    asset_infos.clone(),
                    None,
                    None,
                    lock_filter,
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?;
            if live_acps.is_empty() {
                return Ok(OutputCapacityProvider::From);
            }
        }

        Ok(OutputCapacityProvider::To)
    }

    #[tracing_async]
    pub(crate) async fn prebuild_capacity_balance_tx(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        to_items: Vec<String>,
        since: Option<SinceConfig>,
        pay_fee: Option<PayFee>,
        fee: u64,
        mut transfer_components: utils_types::TransferComponents,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // balance capacity
        self.balance_transfer_tx_capacity(
            ctx.clone(),
            from_items,
            &mut transfer_components,
            if pay_fee == Some(PayFee::From) || pay_fee == None {
                Some(fee)
            } else {
                None
            },
        )
        .await?;

        // balance capacity for fee
        if pay_fee == Some(PayFee::To) {
            let to_items = to_items.into_iter().map(Item::Address).collect();
            self.balance_transfer_tx_capacity_fee_by_output(
                to_items,
                &mut transfer_components,
                fee,
            )
            .await?;
        }

        // build tx
        let fee_change_cell_index = transfer_components
            .fee_change_cell_index
            .ok_or(CoreError::InvalidFeeChange)?;
        self.complete_prebuild_transaction(transfer_components, since)
            .map(|(tx_view, script_groups)| (tx_view, script_groups, fee_change_cell_index))
    }

    #[tracing_async]
    pub(crate) async fn build_sudt_type_script(
        &self,
        ctx: Context,
        script_hash: H160,
    ) -> InnerResult<packed::Script> {
        let res = self
            .storage
            .get_scripts(ctx, vec![script_hash], vec![], None, vec![])
            .await
            .map_err(|err| CoreError::DBError(err.to_string()))?
            .get(0)
            .cloned()
            .ok_or(CoreError::CannotGetScriptByHash)?;

        Ok(res)
    }

    pub(crate) fn complete_prebuild_transaction(
        &self,
        components: TransferComponents,
        payload_since: Option<SinceConfig>,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>)> {
        let cell_deps = self.build_cell_deps(components.script_deps)?;
        let inputs = self.build_transfer_tx_cell_inputs(
            &components.inputs,
            payload_since,
            components.dao_since_map,
        )?;
        let script_groups =
            build_script_groups(components.inputs.iter(), components.outputs.iter());
        let witnesses = build_witnesses(
            components.inputs.len(),
            &script_groups,
            &components.inputs_not_require_signature,
            &components.type_witness_args,
        );
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .inputs(inputs)
            .outputs(components.outputs)
            .outputs_data(components.outputs_data)
            .cell_deps(cell_deps)
            .header_deps(components.header_deps)
            .witnesses(witnesses)
            .build();
        Ok((tx_view, script_groups))
    }

    pub(crate) fn update_tx_view_change_cell_by_index(
        &self,
        tx_view: JsonTransactionView,
        change_fee_cell_index: usize,
        estimate_fee: u64,
        actual_fee: u64,
    ) -> InnerResult<JsonTransactionView> {
        let mut tx = tx_view.inner;
        let output = &mut tx.outputs[change_fee_cell_index];

        let change_cell_capacity: u64 = output.capacity.into();
        let updated_change_cell_capacity = change_cell_capacity + estimate_fee - actual_fee;
        let change_cell_type: Option<packed::Script> = output.type_.clone().map(Into::into);
        let change_cell_lock: packed::Script = output.lock.clone().into();
        let updated_change_cell = packed::CellOutputBuilder::default()
            .lock(change_cell_lock)
            .type_(change_cell_type.pack())
            .capacity(updated_change_cell_capacity.pack())
            .build();
        *output = updated_change_cell.into();
        let updated_tx = packed::Transaction::from(tx);
        let raw_updated_tx = updated_tx.raw();
        let updated_tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .cell_deps(raw_updated_tx.cell_deps())
            .header_deps(raw_updated_tx.header_deps())
            .inputs(raw_updated_tx.inputs())
            .outputs(raw_updated_tx.outputs())
            .outputs_data(raw_updated_tx.outputs_data())
            .witnesses(updated_tx.witnesses())
            .build();
        Ok(updated_tx_view.into())
    }

    fn build_cell_deps(&self, script_set: BTreeSet<String>) -> InnerResult<Vec<packed::CellDep>> {
        script_set
            .iter()
            .map(|s| {
                self.builtin_scripts
                    .get(s)
                    .ok_or_else(|| CoreError::MissingScriptInfo(s.clone()).into())
                    .map(|script_info| script_info.cell_dep.to_owned())
            })
            .collect::<Result<Vec<packed::CellDep>, _>>()
    }

    pub(crate) fn build_transfer_tx_cell_inputs(
        &self,
        inputs: &[DetailedCell],
        payload_since: Option<SinceConfig>,
        dao_since_map: HashMap<usize, u64>,
    ) -> InnerResult<Vec<packed::CellInput>> {
        let payload_since = if let Some(config) = payload_since {
            to_since(config)?
        } else {
            0u64
        };
        let inputs: Vec<packed::CellInput> = inputs
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let since = if let Some(since) = dao_since_map.get(&index) {
                    *since
                } else {
                    payload_since
                };
                packed::CellInputBuilder::default()
                    .since(since.pack())
                    .previous_output(cell.out_point.clone())
                    .build()
            })
            .collect();
        Ok(inputs)
    }

    #[tracing_async]
    pub(crate) async fn inner_build_sudt_issue_transaction(
        &self,
        ctx: Context,
        mut payload: SudtIssuePayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.to.is_empty() {
            return Err(CoreError::NeedAtLeastOneTo.into());
        }
        if payload.to.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        if payload.from.is_empty() {
            return Err(CoreError::NeedAtLeastOneFrom.into());
        }
        if payload.from.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        for to_info in &payload.to {
            if 0u128 == to_info.amount.into() {
                return Err(CoreError::AmountMustPositive.into());
            }
        }
        dedup_json_items(&mut payload.from);
        if !self
            .is_items_contain_addresses(&payload.from, &[payload.owner.to_owned()])
            .await?
        {
            return Err(CoreError::FromNotContainOwner.into());
        }
        self.build_transaction_with_adjusted_fee(
            Self::prebuild_sudt_issue_transaction,
            ctx.clone(),
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_sudt_issue_transaction(
        &self,
        ctx: Context,
        payload: SudtIssuePayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        match &payload.output_capacity_provider {
            Some(OutputCapacityProvider::From) => {
                self.prebuild_sudt_issue_transaction_from_provide_capacity(ctx, payload, fixed_fee)
                    .await
            }
            None | Some(OutputCapacityProvider::To) => {
                self.prebuild_sudt_issue_transaction_to_provide_capacity(ctx, payload, fixed_fee)
                    .await
            }
        }
    }

    #[tracing_async]
    async fn prebuild_sudt_issue_transaction_from_provide_capacity(
        &self,
        ctx: Context,
        payload: SudtIssuePayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build cheque outputs
        let mut transfer_components = utils_types::TransferComponents::new();

        for to in &payload.to {
            let to_address = Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            let to_lock = address_to_script(to_address.payload());

            let owner_address =
                Address::from_str(&payload.owner).map_err(CoreError::InvalidRpcParams)?;
            let owner_script = address_to_script(owner_address.payload());
            let sudt_type_script = self
                .get_script_builder(SUDT)?
                .args(owner_script.calc_script_hash().raw_data().pack())
                .build();
            let sudt_type_script = Some(sudt_type_script).pack();
            let udt_amount = to.amount.into();
            let capacity = calculate_cell_capacity(
                &to_lock,
                &sudt_type_script,
                Capacity::bytes(Bytes::from(encode_udt_amount(udt_amount)).len())
                    .expect("generate capacity"),
            );

            build_cell_for_output(
                capacity,
                to_lock,
                sudt_type_script.to_opt(),
                Some(udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
            transfer_components.script_deps.insert(SUDT.to_string());
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            vec![],
            payload.since,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_sudt_issue_transaction_to_provide_capacity(
        &self,
        ctx: Context,
        payload: SudtIssuePayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        let owner_address =
            Address::from_str(&payload.owner).map_err(CoreError::InvalidRpcParams)?;
        let owner_script = address_to_script(owner_address.payload());
        let sudt_type_script = self
            .get_script_builder(SUDT)?
            .args(owner_script.calc_script_hash().raw_data().pack())
            .build();
        let mut asset_set = HashSet::new();
        asset_set.insert(AssetInfo::new_udt(
            sudt_type_script.calc_script_hash().unpack(),
        ));

        for to in &payload.to {
            let to_address =
                Address::from_str(&to.address).map_err(CoreError::ParseAddressError)?;
            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    Item::Address(to_address.to_string()),
                    asset_set.clone(),
                    None,
                    None,
                    None,
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?;
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let live_acp = live_acps[0].clone();
            let existing_udt_amount = decode_udt_amount(&live_acps[0].cell_data).unwrap_or(0);
            transfer_components.inputs.push(live_acps[0].clone());
            transfer_components
                .inputs_not_require_signature
                .insert(transfer_components.inputs.len() - 1);

            if is_acp(&to_address) {
                transfer_components.script_deps.insert(ACP.to_string());
            }
            if is_pw_lock(&to_address) {
                transfer_components
                    .script_deps
                    .insert(SECP256K1.to_string());
                transfer_components.script_deps.insert(PW_LOCK.to_string());
            }
            transfer_components.script_deps.insert(SUDT.to_string());

            // build acp output
            let to_udt_amount: u128 = to.amount.into();
            build_cell_for_output(
                live_acp.cell_output.capacity().unpack(),
                live_acp.cell_output.lock(),
                live_acp.cell_output.type_().to_opt(),
                Some(existing_udt_amount + to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx,
            map_json_items(payload.from)?,
            vec![],
            payload.since,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }
}

fn map_json_items(json_items: Vec<JsonItem>) -> InnerResult<Vec<Item>> {
    let items = json_items
        .into_iter()
        .map(Item::try_from)
        .collect::<Result<Vec<Item>, _>>()?;
    Ok(items)
}

pub(crate) fn calculate_tx_size(tx_view: &TransactionView) -> usize {
    let tx_size = tx_view.data().total_size();
    // tx offset bytesize
    tx_size + 4
}

fn build_script_groups(
    tx_inputs: Iter<DetailedCell>,
    tx_outputs: Iter<packed::CellOutput>,
) -> Vec<ScriptGroup> {
    let mut script_groups: HashMap<(packed::Script, ScriptGroupType), ScriptGroup> =
        HashMap::default();
    tx_inputs.enumerate().for_each(|(i, cell)| {
        let lock_script = cell.cell_output.lock();
        let lock_group_entry = script_groups
            .entry((lock_script.clone(), ScriptGroupType::Lock))
            .or_insert_with(|| ScriptGroup {
                script: lock_script.into(),
                group_type: ScriptGroupType::Lock,
                input_indices: vec![],
                output_indices: vec![],
            });
        lock_group_entry.input_indices.push((i as u32).into());
        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_group_entry = script_groups
                .entry((type_script.clone(), ScriptGroupType::Type))
                .or_insert_with(|| ScriptGroup {
                    script: type_script.into(),
                    group_type: ScriptGroupType::Type,
                    input_indices: vec![],
                    output_indices: vec![],
                });
            type_group_entry.input_indices.push((i as u32).into());
        }
    });
    tx_outputs.enumerate().for_each(|(i, cell)| {
        if let Some(type_script) = cell.type_().to_opt() {
            let type_group_entry = script_groups
                .entry((type_script.clone(), ScriptGroupType::Type))
                .or_insert_with(|| ScriptGroup {
                    script: type_script.into(),
                    group_type: ScriptGroupType::Type,
                    input_indices: vec![],
                    output_indices: vec![],
                });
            type_group_entry.output_indices.push((i as u32).into());
        }
    });
    script_groups.values().cloned().collect()
}

fn build_witnesses(
    inputs_len: usize,
    script_groups: &[ScriptGroup],
    inputs_not_require_signature: &HashSet<usize>,
    type_witness_args: &HashMap<usize, (packed::BytesOpt, packed::BytesOpt)>,
) -> Vec<packed::Bytes> {
    let mut witnesses = vec![packed::Bytes::default(); inputs_len];
    for script_group in script_groups {
        if script_group.group_type == ScriptGroupType::Type {
            continue;
        }
        let input_index: u32 = script_group.input_indices[0].into();
        if inputs_not_require_signature
            .get(&(input_index as usize))
            .is_some()
        {
            continue;
        }
        // Currently, the length of the placeholder is hard-coded to 65, which is just enough to support lock scripts such as secp, anyone can pay, cheque, and pw lock.
        // In the future, a more general approach will be supported, and placeholder len will be set according to what is in the script group.
        let mut placeholder = packed::WitnessArgs::new_builder()
            .lock(Some(Bytes::from(vec![0u8; 65])).pack())
            .build();
        if let Some((input_type, output_type)) = type_witness_args.get(&(input_index as usize)) {
            placeholder = placeholder
                .as_builder()
                .input_type(input_type.to_owned())
                .output_type(output_type.to_owned())
                .build();
        };
        witnesses[input_index as usize] = placeholder.as_bytes().pack();
        for input_index in &script_group.input_indices[1..] {
            let input_index: u32 = (*input_index).into();
            if let Some((input_type, output_type)) = type_witness_args.get(&(input_index as usize))
            {
                let witness = packed::WitnessArgs::new_builder()
                    .input_type(input_type.to_owned())
                    .output_type(output_type.to_owned())
                    .build();
                witnesses[input_index as usize] = witness.as_bytes().pack();
            };
        }
    }
    witnesses
}
