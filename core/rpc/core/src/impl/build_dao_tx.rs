use crate::r#impl::utils::{
    build_cell_for_output, calculate_unlock_epoch_number, dedup_json_items, is_dao_withdraw_unlock,
    map_json_items, to_since,
};
use crate::r#impl::{utils_types, utils_types::TransferComponents};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use ckb_types::core::{EpochNumberWithFraction, ScriptHashType, TransactionView};
use ckb_types::{bytes::Bytes, packed, prelude::*, U256};
use common::address::{is_pw_lock, is_secp256k1};
use common::lazy::{PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH};
use common::{Address, PaginationRequest, DAO, PW_LOCK, SECP256K1};
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{MAX_ITEM_NUM, MIN_DAO_CAPACITY};
use core_rpc_types::lazy::CURRENT_EPOCH_NUMBER;
use core_rpc_types::{
    AssetInfo, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload, ExtraType, Item, LockFilter,
    ScriptGroup, SinceConfig, SinceFlag, SinceType, TransactionCompletionResponse,
};

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::str::FromStr;
use std::vec;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_dao_deposit_transaction(
        &self,
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
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    async fn prebuild_dao_deposit_transaction(
        &self,
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
        self.prebuild_capacity_balance_tx(items, vec![], None, None, fixed_fee, transfer_components)
            .await
    }

    pub(crate) async fn inner_build_dao_withdraw_transaction(
        &self,
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
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    async fn prebuild_dao_withdraw_transaction(
        &self,
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

            let mut lock_filters = HashMap::new();
            if is_secp256k1(&address) {
                lock_filters.insert(
                    SECP256K1_CODE_HASH
                        .get()
                        .expect("get built-in secp code hash"),
                    LockFilter::default(),
                )
            } else if is_pw_lock(&address) {
                lock_filters.insert(
                    PW_LOCK_CODE_HASH
                        .get()
                        .expect("get built-in pw lock code hash"),
                    LockFilter::default(),
                )
            } else {
                continue;
            };

            // get deposit cells
            let mut cells = self
                .get_live_cells_by_item(
                    from_item.clone(),
                    asset_ckb_set.clone(),
                    None,
                    None,
                    lock_filters,
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
            map_json_items(payload.from)?,
            vec![],
            None,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    pub(crate) async fn inner_build_dao_claim_transaction(
        &self,
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
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    async fn prebuild_dao_claim_transaction(
        &self,
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

            let mut lock_filter = HashMap::new();
            if is_secp256k1(&from_address) {
                lock_filter.insert(
                    SECP256K1_CODE_HASH
                        .get()
                        .expect("get built-in secp code hash"),
                    LockFilter::default(),
                );
            } else if is_pw_lock(&from_address) {
                lock_filter.insert(
                    PW_LOCK_CODE_HASH
                        .get()
                        .expect("get built-in pw lock code hash"),
                    LockFilter::default(),
                );
            } else {
                continue;
            };

            // get withdrawing cells including in lock period
            let mut cells = self
                .get_live_cells_by_item(
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
                .inner_get_transaction_with_status(withdrawing_cell.out_point.tx_hash().unpack())
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
}
