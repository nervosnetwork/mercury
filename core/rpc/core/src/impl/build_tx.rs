use crate::r#impl::{address_to_script, utils, utils_types, utils_types::TransferComponents};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{
    EpochNumberWithFraction, ScriptHashType, TransactionBuilder, TransactionView,
};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256, U256};
use common::address::{is_acp, is_pw_lock, is_secp256k1};
use common::hash::blake2b_256_to_160;
use common::lazy::{ACP_CODE_HASH, DAO_CODE_HASH, PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH};
use common::utils::decode_udt_amount;
use common::{
    Address, Context, DetailedCell, PaginationRequest, ACP, CHEQUE, DAO, PW_LOCK, SECP256K1, SUDT,
};
use common_logger::tracing_async;
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{
    BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM,
    MIN_CKB_CAPACITY, MIN_DAO_CAPACITY, STANDARD_SUDT_CAPACITY,
};
use core_rpc_types::lazy::CURRENT_EPOCH_NUMBER;
use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload, ExtraType, From,
    Item, JsonItem, Mode, ScriptGroup, ScriptGroupType, SimpleTransferPayload, SinceConfig,
    SinceFlag, SinceType, SudtIssuePayload, To, ToInfo, TransactionCompletionResponse,
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
        payload: DaoDepositPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.items.is_empty() {
            return Err(CoreError::NeedAtLeastOneFrom.into());
        }
        if payload.from.items.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        if MIN_DAO_CAPACITY > payload.amount.into() {
            return Err(CoreError::InvalidDAOCapacity.into());
        }
        utils::check_same_enum_value(&payload.from.items)?;
        let mut payload = payload;
        utils::dedup_json_items(&mut payload.from.items);

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

        let items = map_json_items(payload.from.items)?;

        // build output deposit cell
        let deposit_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(CoreError::InvalidRpcParams(error).into()),
            },
            None => {
                self.get_default_owner_address_by_item(items[0].clone())
                    .await?
            }
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
            ctx.clone(),
            items,
            None,
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
        payload: DaoWithdrawPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
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

        let from_item = Item::try_from(payload.clone().from)?;
        let address = self
            .get_default_owner_address_by_item(from_item.clone())
            .await
            .expect("impossible: get default address fail");

        // get deposit cells
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());

        let cells = if is_secp256k1(&address) {
            self.get_live_cells_by_item(
                ctx.clone(),
                from_item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                SECP256K1_CODE_HASH.get(),
                Some(ExtraType::Dao),
                &mut PaginationRequest::default(),
            )
            .await?
        } else if is_pw_lock(&address) {
            self.get_live_cells_by_item(
                ctx.clone(),
                from_item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                PW_LOCK_CODE_HASH.get(),
                Some(ExtraType::Dao),
                &mut PaginationRequest::default(),
            )
            .await?
        } else {
            vec![]
        };

        let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
        let deposit_cells = cells
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
        if is_pw_lock(&address) {
            transfer_components.script_deps.insert(PW_LOCK.to_string());
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx.clone(),
            vec![from_item],
            None,
            self.map_option_address_to_identity(payload.pay_fee)?,
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
        payload: DaoClaimPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
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
        let from_item = Item::try_from(payload.clone().from)?;
        let from_address = self
            .get_default_owner_address_by_item(from_item.clone())
            .await?;

        let to_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(CoreError::InvalidRpcParams(error).into()),
            },
            None => from_address.clone(),
        };
        if !(is_secp256k1(&to_address) || is_pw_lock(&to_address)) {
            return Err(CoreError::InvalidRpcParams(
                "Every to address should be secp/256k1 or pw lock address".to_string(),
            )
            .into());
        }

        // get withdrawing cells including in lock period
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        let cells = if is_secp256k1(&from_address) {
            self.get_live_cells_by_item(
                ctx.clone(),
                from_item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                SECP256K1_CODE_HASH.get(),
                Some(ExtraType::Dao),
                &mut PaginationRequest::default(),
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
                &mut PaginationRequest::default(),
            )
            .await?
        } else {
            vec![]
        };

        let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
        let withdrawing_cells = cells
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
        let mut maximum_withdraw_capacity = 0;
        let from_address = self.get_default_owner_address_by_item(from_item).await?;

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

            if !utils::is_dao_withdraw_unlock(
                EpochNumberWithFraction::from_full_value(deposit_cell.epoch_number).to_rational(),
                EpochNumberWithFraction::from_full_value(withdrawing_cell.epoch_number)
                    .to_rational(),
                Some((**CURRENT_EPOCH_NUMBER.load()).clone()),
            ) {
                continue;
            }

            // calculate input since
            let unlock_epoch = utils::calculate_unlock_epoch_number(
                deposit_cell.epoch_number,
                withdrawing_cell.epoch_number,
            );
            let since = utils::to_since(SinceConfig {
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
        let change_cell_index = utils::build_cell_for_output(
            output_cell_capacity,
            to_address.payload().into(),
            None,
            None,
            &mut transfer_components.outputs,
            &mut transfer_components.outputs_data,
        )?;

        // build resp
        transfer_components
            .script_deps
            .insert(SECP256K1.to_string());
        if is_pw_lock(&from_address) {
            transfer_components.script_deps.insert(PW_LOCK.to_string());
        }
        transfer_components.script_deps.insert(DAO.to_string());

        self.complete_prebuild_transaction(transfer_components, None)
            .map(|(tx_view, script_groups)| (tx_view, script_groups, change_cell_index))
    }

    #[tracing_async]
    pub(crate) async fn inner_build_transfer_transaction(
        &self,
        ctx: Context,
        payload: TransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.items.is_empty() || payload.to.to_infos.is_empty() {
            return Err(CoreError::NeedAtLeastOneFromAndOneTo.into());
        }
        if payload.from.items.len() > MAX_ITEM_NUM || payload.to.to_infos.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }
        utils::check_same_enum_value(&payload.from.items)?;
        let mut payload = payload;
        utils::dedup_json_items(&mut payload.from.items);
        self.check_from_contain_to(
            payload.from.items.iter().collect(),
            payload
                .to
                .to_infos
                .iter()
                .map(|to_info| to_info.address.to_owned())
                .collect(),
        )
        .await?;
        for to_info in &payload.to.to_infos {
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
        match (&payload.asset_info.asset_type, &payload.to.mode) {
            (AssetType::CKB, Mode::HoldByFrom) => {
                self.prebuild_ckb_transfer_transaction_hold_by_from(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::CKB, Mode::HoldByTo) => {
                self.prebuild_ckb_transfer_transaction_hold_by_to(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::CKB, Mode::PayWithAcp) => {
                Err(CoreError::UnsupportTransferMode("PayWithAcp".to_string()).into())
            }
            (AssetType::UDT, Mode::HoldByFrom) => {
                self.prebuild_udt_transfer_transaction_hold_by_from(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::HoldByTo) => {
                self.prebuild_udt_transfer_transaction_hold_by_to(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::PayWithAcp) => {
                self.prebuild_udt_transfer_transaction_pay_with_acp(ctx.clone(), payload, fixed_fee)
                    .await
            }
        }
    }

    #[tracing_async]
    async fn prebuild_ckb_transfer_transaction_hold_by_from(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build the outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        for to in &payload.to.to_infos {
            let capacity = to.amount.into();
            if (u64::MAX as u128) < capacity {
                return Err(CoreError::RequiredCKBMoreThanMax.into());
            }
            let capacity = capacity as u64;
            if MIN_CKB_CAPACITY > capacity {
                return Err(CoreError::RequiredCKBLessThanMin.into());
            }
            let to_address = Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            utils::build_cell_for_output(
                capacity,
                to_address.payload().into(),
                None,
                None,
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx.clone(),
            map_json_items(payload.from.items)?,
            payload.since,
            self.map_option_address_to_identity(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_ckb_transfer_transaction_hold_by_to(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = TransferComponents::new();
        for to in &payload.to.to_infos {
            let item = Item::Identity(self.address_to_identity(&to.address)?);
            let to_address =
                Address::from_str(&to.address).map_err(CoreError::ParseAddressError)?;

            // build acp input
            let lock_filter = if is_secp256k1(&to_address) || is_acp(&to_address) {
                ACP_CODE_HASH.get()
            } else if is_pw_lock(&to_address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                return Err(CoreError::CannotFindACPCell.into());
            };

            let live_acps: Vec<DetailedCell> = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    item.clone(),
                    HashSet::new(),
                    None,
                    None,
                    lock_filter,
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
            transfer_components.script_deps.insert(ACP.to_string());
            transfer_components
                .script_deps
                .insert(SECP256K1.to_string());
            transfer_components.script_deps.insert(PW_LOCK.to_string());
            transfer_components.script_deps.insert(SUDT.to_string());

            // build acp output
            let required_capacity: u128 = to.amount.into();
            utils::build_cell_for_output(
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
            ctx.clone(),
            map_json_items(payload.from.items)?,
            payload.since,
            self.map_option_address_to_identity(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_udt_transfer_transaction_hold_by_from(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        for to in &payload.to.to_infos {
            let receiver_address =
                Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            if !is_secp256k1(&receiver_address) {
                return Err(CoreError::InvalidRpcParams(
                    "Every to address should be secp/256k1 address".to_string(),
                )
                .into());
            }

            // build cheque output
            let sudt_type_script = self
                .build_sudt_type_script(
                    ctx.clone(),
                    blake2b_256_to_160(&payload.asset_info.udt_hash),
                )
                .await?;
            let to_udt_amount = to.amount.into();
            let sender_address = {
                let json_item = &payload.from.items[0];
                let item = Item::try_from(json_item.to_owned())?;
                self.get_secp_address_by_item(item).await?
            };
            let cheque_args = utils::build_cheque_args(receiver_address, sender_address);
            let cheque_lock = self
                .get_script_builder(CHEQUE)?
                .args(cheque_args)
                .hash_type(ScriptHashType::Type.into())
                .build();
            utils::build_cell_for_output(
                CHEQUE_CELL_CAPACITY,
                cheque_lock,
                Some(sudt_type_script),
                Some(to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
            transfer_components.script_deps.insert(CHEQUE.to_string());
            transfer_components.script_deps.insert(SUDT.to_string());
        }

        // balance udt
        let from_items = payload
            .from
            .items
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
            ctx.clone(),
            map_json_items(payload.from.items)?,
            payload.since,
            self.map_option_address_to_identity(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_udt_transfer_transaction_hold_by_to(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build acp inputs and outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        let mut asset_set = HashSet::new();
        asset_set.insert(payload.asset_info.clone());

        for to in &payload.to.to_infos {
            let item = Item::Identity(self.address_to_identity(&to.address)?);
            let to_address =
                Address::from_str(&to.address).map_err(CoreError::ParseAddressError)?;

            // build acp input
            let lock_filter = if is_secp256k1(&to_address) || is_acp(&to_address) {
                ACP_CODE_HASH.get()
            } else if is_pw_lock(&to_address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                return Err(CoreError::CannotFindACPCell.into());
            };

            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    item.clone(),
                    asset_set.clone(),
                    None,
                    None,
                    lock_filter,
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
            transfer_components.script_deps.insert(ACP.to_string());
            transfer_components
                .script_deps
                .insert(SECP256K1.to_string());
            transfer_components.script_deps.insert(PW_LOCK.to_string());
            transfer_components.script_deps.insert(SUDT.to_string());

            // build acp output
            let to_udt_amount: u128 = to.amount.into();
            utils::build_cell_for_output(
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
            .items
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
            ctx.clone(),
            map_json_items(payload.from.items)?,
            payload.since,
            self.map_option_address_to_identity(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_udt_transfer_transaction_pay_with_acp(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components
        let mut transfer_components = utils_types::TransferComponents::new();
        for to in &payload.to.to_infos {
            // build acp output
            let to_udt_amount = to.amount.into();
            let to_item = Item::Identity(self.address_to_identity(&to.address)?);
            let to_acp_address = self.get_acp_address_by_item(&to_item).await?;
            let sudt_type_script = self
                .build_sudt_type_script(
                    ctx.clone(),
                    blake2b_256_to_160(&payload.asset_info.udt_hash),
                )
                .await?;
            utils::build_cell_for_output(
                STANDARD_SUDT_CAPACITY,
                to_acp_address.payload().into(),
                Some(sudt_type_script),
                Some(to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }
        transfer_components.script_deps.insert(SUDT.to_string());

        // balance udt
        let from_items = payload
            .from
            .items
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
            ctx.clone(),
            map_json_items(payload.from.items)?,
            payload.since,
            self.map_option_address_to_identity(payload.pay_fee)?,
            payload.change,
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
        utils::dedup_json_items(&mut from_items);
        self.check_from_contain_to(
            from_items.iter().collect(),
            payload
                .to
                .iter()
                .map(|to_info| to_info.address.to_owned())
                .collect(),
        )
        .await?;
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
                    from: From { items: from_items },
                    to: To {
                        to_infos: payload.to,
                        mode: Mode::HoldByFrom,
                    },
                    pay_fee: None,
                    change: payload.change,
                    fee_rate: payload.fee_rate,
                    since: payload.since,
                };
                self.prebuild_ckb_transfer_transaction_hold_by_from(
                    ctx.clone(),
                    transfer_payload,
                    fixed_fee,
                )
                .await
            }

            AssetType::UDT => {
                let mut asset_infos = HashSet::new();
                asset_infos.insert(payload.asset_info.clone());
                let mode = self
                    .get_simple_transfer_mode(ctx.clone(), &to_items, asset_infos.clone())
                    .await?;
                let transfer_payload = TransferPayload {
                    asset_info: payload.asset_info,
                    from: From { items: from_items },
                    to: To {
                        to_infos: payload.to.clone(),
                        mode: mode.clone(),
                    },
                    pay_fee: None,
                    change: payload.change,
                    fee_rate: payload.fee_rate,
                    since: payload.since,
                };
                match mode {
                    Mode::HoldByFrom => {
                        self.prebuild_udt_transfer_transaction_hold_by_from(
                            ctx.clone(),
                            transfer_payload,
                            fixed_fee,
                        )
                        .await
                    }
                    Mode::HoldByTo => {
                        self.prebuild_udt_transfer_transaction_hold_by_to(
                            ctx.clone(),
                            transfer_payload,
                            fixed_fee,
                        )
                        .await
                    }
                    Mode::PayWithAcp => {
                        Err(CoreError::UnsupportTransferMode("PayWithAcp".to_string()).into())
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
            let tx_size = calculate_tx_size(tx_view.clone());
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
    async fn get_simple_transfer_mode(
        &self,
        ctx: Context,
        to_items: &[Item],
        asset_infos: HashSet<AssetInfo>,
    ) -> InnerResult<Mode> {
        for i in to_items {
            let to_address = self.get_default_owner_address_by_item(i.to_owned()).await?;

            let live_acps = if is_secp256k1(&to_address) {
                self.get_live_cells_by_item(
                    ctx.clone(),
                    i.to_owned(),
                    asset_infos.clone(),
                    None,
                    None,
                    ACP_CODE_HASH.get(),
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?
            } else if is_pw_lock(&to_address) {
                self.get_live_cells_by_item(
                    ctx.clone(),
                    i.to_owned(),
                    asset_infos.clone(),
                    None,
                    None,
                    PW_LOCK_CODE_HASH.get(),
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?
            } else {
                vec![]
            };
            if live_acps.is_empty() {
                return Ok(Mode::HoldByFrom);
            }
        }

        Ok(Mode::HoldByTo)
    }

    #[tracing_async]
    pub(crate) async fn prebuild_capacity_balance_tx(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        since: Option<SinceConfig>,
        pay_fee: Option<Item>,
        change: Option<String>,
        fee: u64,
        mut transfer_components: utils_types::TransferComponents,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // balance capacity
        self.balance_transfer_tx_capacity(
            ctx.clone(),
            from_items,
            &mut transfer_components,
            if pay_fee.is_none() { Some(fee) } else { None },
            change,
        )
        .await?;

        // balance capacity for fee
        if let Some(pay_item) = pay_fee {
            let pay_items = vec![pay_item];
            self.balance_transfer_tx_capacity(
                ctx.clone(),
                pay_items,
                &mut transfer_components,
                Some(fee),
                None,
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
        // build cell deps
        let cell_deps = self.build_cell_deps(components.script_deps)?;

        // build inputs
        let inputs = self.build_transfer_tx_cell_inputs(
            &components.inputs,
            payload_since,
            components.dao_since_map,
        )?;

        // build script group
        let script_groups =
            build_script_groups(components.inputs.iter(), components.outputs.iter());

        // build witnesses
        let mut witnesses = vec![packed::Bytes::default(); components.inputs.len()];
        for script_group in &script_groups {
            if script_group.group_type == ScriptGroupType::TypeScript {
                continue;
            }
            let input_index: u32 = script_group.input_indices[0].into();
            if components
                .inputs_not_require_signature
                .get(&(input_index as usize))
                .is_some()
            {
                continue;
            }
            let mut placeholder = packed::WitnessArgs::new_builder()
                .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                .build();
            if let Some((input_type, output_type)) =
                components.type_witness_args.get(&(input_index as usize))
            {
                placeholder = placeholder
                    .as_builder()
                    .input_type(input_type.to_owned())
                    .output_type(output_type.to_owned())
                    .build();
            };
            witnesses[input_index as usize] = placeholder.as_bytes().pack();
            for input_index in &script_group.input_indices[1..] {
                let input_index: u32 = (*input_index).into();
                if let Some((input_type, output_type)) =
                    components.type_witness_args.get(&(input_index as usize))
                {
                    let witness = packed::WitnessArgs::new_builder()
                        .input_type(input_type.to_owned())
                        .output_type(output_type.to_owned())
                        .build();
                    witnesses[input_index as usize] = witness.as_bytes().pack();
                };
            }
        }

        // build tx view
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
            utils::to_since(config)?
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
        payload: SudtIssuePayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.to.to_infos.is_empty() {
            return Err(CoreError::NeedAtLeastOneTo.into());
        }

        if payload.to.to_infos.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }

        for to_info in &payload.to.to_infos {
            if 0u128 == to_info.amount.into() {
                return Err(CoreError::TransferAmountMustPositive.into());
            }
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
        match &payload.to.mode {
            Mode::HoldByFrom => {
                self.prebuild_cheque_sudt_issue_transaction(ctx.clone(), payload, fixed_fee)
                    .await
            }
            Mode::HoldByTo => {
                self.prebuild_acp_sudt_issue_transaction(ctx.clone(), payload, fixed_fee)
                    .await
            }
            Mode::PayWithAcp => {
                Err(CoreError::UnsupportTransferMode("PayWithAcp".to_string()).into())
            }
        }
    }

    #[tracing_async]
    async fn prebuild_cheque_sudt_issue_transaction(
        &self,
        ctx: Context,
        payload: SudtIssuePayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        // init transfer components: build cheque outputs
        let mut transfer_components = utils_types::TransferComponents::new();
        let owner_item = Item::Address(payload.owner.to_owned());

        for to in &payload.to.to_infos {
            let receiver_address =
                Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            if !is_secp256k1(&receiver_address) {
                return Err(CoreError::InvalidRpcParams(
                    "Every to address should be secp/256k1 address".to_string(),
                )
                .into());
            }

            // build cheque output
            let owner_address =
                Address::from_str(&payload.owner).map_err(CoreError::InvalidRpcParams)?;
            let owner_script = address_to_script(owner_address.payload());
            let sudt_type_script = self
                .get_script_builder(SUDT)?
                .args(owner_script.calc_script_hash().raw_data().pack())
                .build();
            let to_udt_amount = to.amount.into();
            let sender_address = self.get_secp_address_by_item(owner_item.clone()).await?;
            let cheque_args = utils::build_cheque_args(receiver_address, sender_address);
            let cheque_lock = self
                .get_script_builder(CHEQUE)?
                .args(cheque_args)
                .hash_type(ScriptHashType::Type.into())
                .build();
            utils::build_cell_for_output(
                CHEQUE_CELL_CAPACITY,
                cheque_lock,
                Some(sudt_type_script),
                Some(to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
            transfer_components.script_deps.insert(CHEQUE.to_string());
            transfer_components.script_deps.insert(SUDT.to_string());
        }

        // balance capacity
        self.prebuild_capacity_balance_tx(
            ctx.clone(),
            vec![owner_item],
            payload.since,
            map_option_json_item(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_acp_sudt_issue_transaction(
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

        for to in &payload.to.to_infos {
            let item = Item::Identity(self.address_to_identity(&to.address)?);
            let to_address =
                Address::from_str(&to.address).map_err(CoreError::ParseAddressError)?;

            // build acp input
            let lock_filter = if is_secp256k1(&to_address) || is_acp(&to_address) {
                ACP_CODE_HASH.get()
            } else if is_pw_lock(&to_address) {
                PW_LOCK_CODE_HASH.get()
            } else {
                return Err(CoreError::CannotFindACPCell.into());
            };

            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
                    item.clone(),
                    asset_set.clone(),
                    None,
                    None,
                    lock_filter,
                    None,
                    &mut PaginationRequest::default().limit(Some(1)),
                )
                .await?;
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let existing_udt_amount = decode_udt_amount(&live_acps[0].cell_data).unwrap_or(0);
            transfer_components.inputs.push(live_acps[0].clone());
            transfer_components
                .inputs_not_require_signature
                .insert(transfer_components.inputs.len() - 1);
            transfer_components.script_deps.insert(ACP.to_string());
            transfer_components.script_deps.insert(SUDT.to_string());
            transfer_components
                .script_deps
                .insert(SECP256K1.to_string());
            transfer_components.script_deps.insert(PW_LOCK.to_string());

            // build acp output
            let to_udt_amount: u128 = to.amount.into();
            utils::build_cell_for_output(
                live_acps[0].cell_output.capacity().unpack(),
                live_acps[0].cell_output.lock(),
                live_acps[0].cell_output.type_().to_opt(),
                Some(existing_udt_amount + to_udt_amount),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        let owner_item = Item::Address(payload.owner.to_owned());
        self.prebuild_capacity_balance_tx(
            ctx.clone(),
            vec![owner_item],
            payload.since,
            map_option_json_item(payload.pay_fee)?,
            payload.change,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    fn map_option_address_to_identity(&self, address: Option<String>) -> InnerResult<Option<Item>> {
        Ok(match address {
            Some(addr) => Some(Item::Identity(self.address_to_identity(&addr)?)),
            None => None,
        })
    }
}

fn map_json_items(json_items: Vec<JsonItem>) -> InnerResult<Vec<Item>> {
    let items = json_items
        .into_iter()
        .map(Item::try_from)
        .collect::<Result<Vec<Item>, _>>()?;
    Ok(items)
}

fn map_option_json_item(json_item: Option<JsonItem>) -> InnerResult<Option<Item>> {
    Ok(match json_item {
        Some(item) => Some(Item::try_from(item)?),
        None => None,
    })
}

pub(crate) fn calculate_tx_size(tx_view: TransactionView) -> usize {
    let tx_size = tx_view.data().total_size();
    // tx offset bytesize
    tx_size + 4
}

fn build_script_groups(
    tx_inputs: Iter<DetailedCell>,
    tx_outputs: Iter<packed::CellOutput>,
) -> Vec<ScriptGroup> {
    let mut script_group_index_map: HashMap<H256, usize> = HashMap::new();
    let mut script_groups: Vec<ScriptGroup> = vec![];
    tx_inputs.enumerate().for_each(|(input_index, cell)| {
        let input_index = input_index as u32;
        let lock_script = cell.cell_output.lock();
        let lock_hash = lock_script.calc_script_hash().unpack();
        if let Some(script_group_index) = script_group_index_map.get_mut(&lock_hash) {
            script_groups[*script_group_index].add_group_inputs(input_index);
        } else {
            script_groups.push(ScriptGroup {
                script: lock_script.into(),
                group_type: ScriptGroupType::LockScript,
                input_indices: vec![input_index.into()],
                output_indices: vec![],
            });
            script_group_index_map.insert(lock_hash, script_groups.len() - 1);
        }
        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_hash = type_script.calc_script_hash().unpack();
            if let Some(script_group_index) = script_group_index_map.get_mut(&type_hash) {
                script_groups[*script_group_index].add_group_inputs(input_index);
            } else {
                script_groups.push(ScriptGroup {
                    script: type_script.into(),
                    group_type: ScriptGroupType::TypeScript,
                    input_indices: vec![input_index.into()],
                    output_indices: vec![],
                });
                script_group_index_map.insert(type_hash, script_groups.len() - 1);
            }
        }
    });
    tx_outputs.enumerate().for_each(|(output_index, cell)| {
        if let Some(type_script) = cell.type_().to_opt() {
            let output_index = output_index as u32;
            let type_hash = type_script.calc_script_hash().unpack();
            if let Some(script_group_index) = script_group_index_map.get_mut(&type_hash) {
                script_groups[*script_group_index].add_group_outputs(output_index);
            } else {
                script_groups.push(ScriptGroup {
                    script: type_script.into(),
                    group_type: ScriptGroupType::TypeScript,
                    input_indices: vec![],
                    output_indices: vec![output_index.into()],
                });
                script_group_index_map.insert(type_hash, script_groups.len() - 1);
            }
        }
    });
    script_groups
}
