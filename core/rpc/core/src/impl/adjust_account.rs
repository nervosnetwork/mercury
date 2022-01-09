use crate::r#impl::{calculate_tx_size, utils, utils_types};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{ckb, DEFAULT_FEE_RATE, STANDARD_SUDT_CAPACITY};
use core_rpc_types::lazy::{ACP_CODE_HASH, SECP256K1_CODE_HASH};
use core_rpc_types::{
    AdjustAccountPayload, AssetType, HashAlgorithm, Item, JsonItem, SignAlgorithm, SignatureAction,
    Source, TransactionCompletionResponse,
};

use common::hash::blake2b_256_to_160;
use common::utils::decode_udt_amount;
use common::{
    Address, AddressPayload, Context, DetailedCell, PaginationRequest, ACP, SECP256K1, SUDT,
};

use ckb_types::core::TransactionView;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160};

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_adjust_account_transaction(
        &self,
        ctx: Context,
        payload: AdjustAccountPayload,
    ) -> InnerResult<Option<TransactionCompletionResponse>> {
        if payload.asset_info.asset_type == AssetType::CKB {
            return Err(CoreError::AdjustAccountOnCkb.into());
        }
        utils::check_same_enum_value(&payload.from.clone().into_iter().collect::<Vec<JsonItem>>())?;

        let account_number = payload.account_number.unwrap_or(1) as usize;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);
        let item: Item = payload.item.clone().try_into()?;

        let mut asset_set = HashSet::new();
        asset_set.insert(payload.asset_info.clone());
        let live_acps = self
            .get_live_cells_by_item(
                ctx.clone(),
                item.clone(),
                asset_set,
                None,
                None,
                Some((**ACP_CODE_HASH.load()).clone()),
                None,
                false,
                &mut PaginationRequest::default(),
            )
            .await?;
        let live_acps_len = live_acps.len();

        if live_acps_len == account_number {
            return Ok(None);
        }

        if live_acps_len < account_number {
            self.build_transaction_with_adjusted_fee(
                |rpc, ctx, payload, fixed_fee| {
                    Self::build_create_acp_transaction_fixed_fee(
                        rpc,
                        ctx,
                        account_number - live_acps_len,
                        payload,
                        fixed_fee,
                    )
                },
                ctx.clone(),
                payload.clone(),
                payload.fee_rate,
            )
            .await
            .map(Some)
        } else {
            let res = self
                .build_collect_asset_transaction_fixed_fee(
                    live_acps,
                    live_acps_len - account_number,
                    fee_rate,
                )
                .await?;

            Ok(Some(TransactionCompletionResponse::new(res.0, res.1)))
        }
    }

    async fn build_create_acp_transaction_fixed_fee(
        &self,
        ctx: Context,
        acp_need_count: usize,
        payload: AdjustAccountPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut transfer_components = utils_types::TransferComponents::new();

        transfer_components.script_deps.insert(ACP.to_string());
        transfer_components.script_deps.insert(SUDT.to_string());

        let item: Item = payload.item.clone().try_into()?;
        let from = parse_from(payload.from.clone())?;
        let extra_ckb = payload.extra_ckb.unwrap_or_else(|| ckb(1));

        let sudt_type_script = self
            .build_sudt_type_script(
                ctx.clone(),
                blake2b_256_to_160(&payload.asset_info.udt_hash),
            )
            .await?;

        for _i in 0..acp_need_count {
            let lock_args = self.get_secp_lock_args_by_item(item.clone())?.0;
            let lock_script = self
                .builtin_scripts
                .get(ACP)
                .cloned()
                .expect("Impossible: get built in script fail")
                .script
                .as_builder()
                .args(lock_args.pack())
                .build();
            utils::build_cell_for_output(
                STANDARD_SUDT_CAPACITY + extra_ckb,
                lock_script,
                Some(sudt_type_script.clone()),
                Some(0),
                &mut transfer_components.outputs,
                &mut transfer_components.outputs_data,
            )?;
        }

        // balance capacity
        let from = if from.is_empty() { vec![item] } else { from };
        self.prebuild_capacity_balance_tx(
            ctx.clone(),
            from,
            None,
            None,
            None,
            Source::Free,
            fixed_fee,
            transfer_components,
        )
        .await
    }

    async fn build_collect_asset_transaction_fixed_fee(
        &self,
        mut acp_cells: Vec<DetailedCell>,
        acp_consume_count: usize,
        fee_rate: u64,
    ) -> InnerResult<(ckb_jsonrpc_types::TransactionView, Vec<SignatureAction>)> {
        if acp_consume_count > acp_cells.len() {
            return Err(CoreError::InvalidAdjustAccountNumber.into());
        }

        let (inputs, output) = if acp_consume_count == acp_cells.len() {
            let inputs = acp_cells;
            let mut tmp = inputs.get(0).cloned().unwrap();
            let args = tmp.cell_output.lock().args().raw_data()[0..20].to_vec();
            let lock_script = tmp
                .cell_output
                .lock()
                .as_builder()
                .code_hash((**SECP256K1_CODE_HASH.load()).clone().pack())
                .args(args.pack())
                .build();
            let type_script: Option<packed::Script> = None;
            let cell = tmp
                .cell_output
                .as_builder()
                .lock(lock_script)
                .type_(type_script.pack())
                .build();
            tmp.cell_output = cell;
            (inputs, tmp)
        } else {
            let _ = acp_cells.split_off(acp_consume_count + 1);

            let inputs = acp_cells;
            let output = inputs.get(0).cloned().unwrap();

            (inputs, output)
        };

        let mut input_capacity_sum = 0;
        let mut input_udt_sum = 0;

        for cell in inputs.iter() {
            let capacity: u64 = cell.cell_output.capacity().unpack();
            let amount = decode_udt_amount(&cell.cell_data);
            input_capacity_sum += capacity;
            input_udt_sum += amount;
        }

        let output_data = if acp_consume_count == inputs.len() {
            if input_udt_sum != 0 {
                return Err(CoreError::NotZeroInputUDTAmount.into());
            }
            Bytes::new()
        } else {
            Bytes::from(input_udt_sum.to_le_bytes().to_vec())
        };
        let output = output
            .cell_output
            .as_builder()
            .capacity((input_capacity_sum).pack())
            .build();

        let mut script_set = HashSet::new();
        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());
        script_set.insert(ACP.to_string());

        let pub_key = H160::from_slice(&output.lock().args().raw_data()[0..20]).unwrap();
        let address = Address::new(
            self.network_type,
            AddressPayload::from_pubkey_hash(pub_key),
            true,
        )
        .to_string();
        let mut signature_actions = HashMap::new();
        for (i, input) in inputs.iter().enumerate() {
            utils::add_signature_action(
                address.clone(),
                input.cell_output.calc_lock_hash().to_string(),
                SignAlgorithm::Secp256k1,
                HashAlgorithm::Blake2b,
                &mut signature_actions,
                i,
            );
        }

        let inputs = inputs
            .iter()
            .map(|input| {
                packed::CellInputBuilder::default()
                    .since(0u64.pack())
                    .previous_output(input.out_point.clone())
                    .build()
            })
            .collect();

        let (tx_view, signature_actions) = self.prebuild_tx_complete(
            inputs,
            vec![output],
            vec![output_data.pack()],
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )?;

        let tx_size = calculate_tx_size(tx_view.clone());
        let actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;

        let tx_view = self.update_tx_view_change_cell_by_index(tx_view.into(), 0, 0, actual_fee)?;
        Ok((tx_view, signature_actions))
    }
}

fn parse_from(from_set: HashSet<JsonItem>) -> InnerResult<Vec<Item>> {
    let mut ret: Vec<Item> = Vec::new();
    for ji in from_set.into_iter() {
        ret.push(ji.try_into()?);
    }

    Ok(ret)
}
