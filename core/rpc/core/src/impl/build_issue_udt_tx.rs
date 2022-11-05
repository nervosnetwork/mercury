use crate::r#impl::utils::{
    build_cell_for_output, calculate_cell_capacity, dedup_json_items, map_json_items,
};
use crate::r#impl::{address_to_script, utils_types};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use ckb_types::core::{Capacity, TransactionView};
use ckb_types::{bytes::Bytes, prelude::*};
use common::address::{is_acp, is_pw_lock};
use common::utils::{decode_udt_amount, encode_udt_amount};
use common::{Address, PaginationRequest, ACP, PW_LOCK, SECP256K1, SUDT};
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::MAX_ITEM_NUM;
use core_rpc_types::{
    AssetInfo, Item, OutputCapacityProvider, ScriptGroup, SudtIssuePayload,
    TransactionCompletionResponse,
};

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::vec;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_sudt_issue_transaction(
        &self,
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
            payload.clone(),
            payload.fee_rate.map(Into::into),
        )
        .await
    }

    async fn prebuild_sudt_issue_transaction(
        &self,
        payload: SudtIssuePayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<ScriptGroup>, usize)> {
        match &payload.output_capacity_provider {
            Some(OutputCapacityProvider::From) => {
                self.prebuild_sudt_issue_transaction_from_provide_capacity(payload, fixed_fee)
                    .await
            }
            None | Some(OutputCapacityProvider::To) => {
                self.prebuild_sudt_issue_transaction_to_provide_capacity(payload, fixed_fee)
                    .await
            }
        }
    }

    async fn prebuild_sudt_issue_transaction_from_provide_capacity(
        &self,
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
            map_json_items(payload.from)?,
            vec![],
            payload.since,
            None,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    async fn prebuild_sudt_issue_transaction_to_provide_capacity(
        &self,
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
                    Item::Address(to_address.to_string()),
                    asset_set.clone(),
                    None,
                    None,
                    HashMap::new(),
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
