use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{calculate_tx_size, ckb, utils, INIT_ESTIMATE_FEE};
use crate::rpc_impl::{
    ACP_CODE_HASH, BYTE_SHANNONS, DEFAULT_FEE_RATE, MIN_CKB_CAPACITY, SECP256K1_CODE_HASH,
    STANDARD_SUDT_CAPACITY,
};
use crate::types::{
    AdjustAccountPayload, AssetType, HashAlgorithm, Item, JsonItem, SignAlgorithm, SignatureAction,
    TransactionCompletionResponse,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::hash::blake2b_256_to_160;
use common::utils::{decode_udt_amount, encode_udt_amount};
use common::{Address, AddressPayload, Context, DetailedCell, ACP, SECP256K1, SUDT};
use common_logger::tracing_async;

use ckb_types::core::TransactionView;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160};

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    #[tracing_async]
    pub(crate) async fn inner_build_adjust_account_transaction(
        &self,
        ctx: Context,
        payload: AdjustAccountPayload,
    ) -> InnerResult<Option<TransactionCompletionResponse>> {
        if payload.asset_info.asset_type == AssetType::CKB {
            return Err(RpcErrorMessage::AdjustAccountOnCkb);
        }
        utils::check_same_enum_value(payload.from.iter().collect())?;

        let account_number = payload.account_number.unwrap_or(1) as usize;
        let extra_ckb = payload.extra_ckb.unwrap_or_else(|| ckb(1));
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);
        let item: Item = payload.item.clone().try_into()?;
        let from = parse_from(payload.from.clone())?;

        let mut estimate_fee = INIT_ESTIMATE_FEE;
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
            )
            .await?;
        let live_acps_len = live_acps.len();

        if live_acps_len == account_number {
            return Ok(None);
        }

        let sudt_type_script = self
            .build_sudt_type_script(
                ctx.clone(),
                blake2b_256_to_160(&payload.asset_info.udt_hash),
            )
            .await?;

        if live_acps_len < account_number {
            loop {
                let (tx_view, signature_actions, change_cell_index) = self
                    .build_create_acp_transaction_fixed_fee(
                        ctx.clone(),
                        from.clone(),
                        account_number - live_acps_len,
                        sudt_type_script.clone(),
                        item.clone(),
                        extra_ckb,
                        estimate_fee,
                    )
                    .await?;
                let tx_size = calculate_tx_size(tx_view.clone()) as u64;
                let mut actual_fee = fee_rate.saturating_mul(tx_size) / 1000;
                if actual_fee * 1000 < fee_rate.saturating_mul(tx_size) {
                    actual_fee += 1;
                }
                if estimate_fee < actual_fee {
                    estimate_fee += BYTE_SHANNONS;
                    continue;
                } else {
                    let tx_view = self.update_tx_view_change_cell_by_index(
                        tx_view.into(),
                        change_cell_index,
                        estimate_fee,
                        actual_fee,
                    )?;
                    let adjust_response =
                        TransactionCompletionResponse::new(tx_view, signature_actions);
                    return Ok(Some(adjust_response));
                }
            }
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

    #[tracing_async]
    async fn build_create_acp_transaction_fixed_fee(
        &self,
        ctx: Context,
        from: Vec<Item>,
        acp_need_count: usize,
        sudt_type_script: packed::Script,
        item: Item,
        extra_ckb: u64,
        fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut ckb_needs = fee + MIN_CKB_CAPACITY;
        let mut outputs_data: Vec<packed::Bytes> = Vec::new();
        let mut outputs = Vec::new();
        let mut input_index = 0;

        for _i in 0..acp_need_count {
            let capacity = STANDARD_SUDT_CAPACITY + extra_ckb;
            let output_cell = self.build_acp_cell(
                Some(sudt_type_script.clone()),
                self.get_secp_lock_args_by_item(item.clone())?.0.to_vec(),
                capacity,
            );

            outputs.push(output_cell);
            outputs_data.push(Bytes::from(encode_udt_amount(0)).pack());
            ckb_needs += capacity;
        }

        let mut inputs = Vec::new();
        let mut input_capacity_sum = 0u64;
        let mut script_set = HashSet::new();
        let mut signature_actions = HashMap::new();
        let from = if from.is_empty() { vec![item] } else { from };

        self.pool_live_cells_by_items(
            ctx.clone(),
            from.clone(),
            ckb_needs,
            vec![],
            None,
            &mut input_capacity_sum,
            &mut inputs,
            &mut script_set,
            &mut signature_actions,
            &mut input_index,
        )
        .await?;

        script_set.insert(ACP.to_string());
        script_set.insert(SUDT.to_string());

        let change_cell = {
            let lock_args = self.get_secp_lock_args_by_item(from[0].clone())?;
            let lock_script = self
                .builtin_scripts
                .get(SECP256K1)
                .cloned()
                .ok_or_else(|| RpcErrorMessage::MissingScriptInfo(SECP256K1.to_string()))?
                .script
                .as_builder()
                .args(lock_args.0.to_vec().pack())
                .build();
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .capacity((input_capacity_sum - ckb_needs + MIN_CKB_CAPACITY).pack())
                .build()
        };

        outputs.push(change_cell);
        outputs_data.push(packed::Bytes::default());
        let change_index = outputs.len() - 1;

        let inputs = inputs
            .iter()
            .map(|input| {
                packed::CellInputBuilder::default()
                    .since(0u64.pack())
                    .previous_output(input.out_point.clone())
                    .build()
            })
            .collect();

        self.prebuild_tx_complete(
            inputs,
            outputs,
            outputs_data,
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_index))
    }

    fn build_acp_cell(
        &self,
        type_script: Option<packed::Script>,
        lock_args: Vec<u8>,
        capacity: u64,
    ) -> packed::CellOutput {
        let lock_script = self
            .builtin_scripts
            .get(ACP)
            .cloned()
            .expect("Impossible: get built in script fail")
            .script
            .as_builder()
            .args(lock_args.pack())
            .build();
        packed::CellOutputBuilder::default()
            .type_(type_script.pack())
            .lock(lock_script)
            .capacity(capacity.pack())
            .build()
    }

    async fn build_collect_asset_transaction_fixed_fee(
        &self,
        mut acp_cells: Vec<DetailedCell>,
        acp_consume_count: usize,
        fee_rate: u64,
    ) -> InnerResult<(ckb_jsonrpc_types::TransactionView, Vec<SignatureAction>)> {
        let acp_need = acp_consume_count + 1;

        if acp_need > acp_cells.len() {
            return Err(RpcErrorMessage::InvalidAdjustAccountNumber);
        }

        let _ = acp_cells.split_off(acp_need);
        let inputs = acp_cells;
        let output = if inputs.len() == 1 {
            let mut tmp = inputs.get(0).cloned().unwrap();
            let args = tmp.cell_output.lock().args().raw_data()[0..20].to_vec();
            let lock_script = tmp
                .cell_output
                .lock()
                .as_builder()
                .code_hash((**SECP256K1_CODE_HASH.load()).clone().pack())
                .args(args.pack())
                .build();
            let cell = tmp.cell_output.as_builder().lock(lock_script).build();
            tmp.cell_output = cell;
            tmp
        } else {
            inputs.get(0).cloned().unwrap()
        };
        let pub_key =
            H160::from_slice(&output.cell_output.lock().args().raw_data()[0..20]).unwrap();

        let mut input_capacity_sum = 0;
        let mut input_udt_sum = 0;

        for cell in inputs.iter() {
            let capacity: u64 = cell.cell_output.capacity().unpack();
            let amount = decode_udt_amount(&cell.cell_data);
            input_capacity_sum += capacity;
            input_udt_sum += amount;
        }

        let output = output
            .cell_output
            .as_builder()
            .capacity((input_capacity_sum).pack())
            .build();
        let output_data = Bytes::from(input_udt_sum.to_le_bytes().to_vec());

        let mut script_set = HashSet::new();
        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());
        script_set.insert(ACP.to_string());

        let address = Address::new(
            self.network_type,
            AddressPayload::from_pubkey_hash(self.network_type, pub_key),
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
