use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{calculate_tx_size_with_witness_placeholder, ckb};
use crate::rpc_impl::{ACP_CODE_HASH, BYTE_SHANNONS, MIN_CKB_CAPACITY, STANDARD_SUDT_CAPACITY};
use crate::types::{
    AdjustAccountPayload, AssetType, Item, JsonItem, SignatureEntry, SignatureType,
    TransactionCompletionResponse, WitnessType,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::decode_udt_amount;
use common::{Address, AddressPayload, DetailedCell, ACP, SECP256K1, SUDT};

use ckb_types::core::{TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160};

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_adjust_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> InnerResult<Option<TransactionCompletionResponse>> {
        if payload.asset_info.asset_type == AssetType::CKB {
            return Err(RpcErrorMessage::AdjustAccountOnCkb);
        }

        let account_number = payload.account_number.unwrap_or(1) as usize;
        let extra_ckb = payload.extra_ckb.unwrap_or_else(|| ckb(1));
        let fee_rate = payload.fee_rate.unwrap_or(1000);
        let item: Item = payload.item.clone().try_into()?;
        let from = parse_from(payload.from.clone())?;
        let mut estimate_fee = ckb(1);

        let mut asset_set = HashSet::new();
        asset_set.insert(payload.asset_info.clone());
        let live_acps = self
            .get_live_cells_by_item(
                item.clone(),
                asset_set,
                None,
                None,
                Some((**ACP_CODE_HASH.load()).clone()),
                None,
            )
            .await?;
        let live_acps_len = live_acps.len();

        if live_acps_len == account_number {
            return Ok(None);
        }

        let is_expand = live_acps_len < account_number;
        let sudt_type_script = self.build_sudt_type_script(payload.asset_info.udt_hash.0.to_vec());

        if is_expand {
            loop {
                let res = self
                    .build_create_acp_transaction_fixed_fee(
                        from.clone(),
                        account_number - live_acps_len,
                        sudt_type_script.clone(),
                        item.clone(),
                        extra_ckb,
                        estimate_fee,
                    )
                    .await?;

                let tx_size =
                    calculate_tx_size_with_witness_placeholder(res.0.clone().into(), res.1.clone())
                        as u64;

                let mut actual_fee = fee_rate.saturating_mul(tx_size) / 1000;
                if actual_fee * 1000 < fee_rate.saturating_mul(tx_size) {
                    actual_fee += 1;
                }

                if estimate_fee < actual_fee {
                    estimate_fee += BYTE_SHANNONS;
                    continue;
                } else {
                    let tx_view = self.update_tx_view_change_cell(
                        res.0.clone().into(),
                        Address::new(self.network_type, res.2.clone()),
                        estimate_fee,
                        actual_fee,
                    )?;
                    let adjust_response = TransactionCompletionResponse::new(tx_view, res.1);
                    return Ok(Some(adjust_response));
                }
            }
        } else {
            let res = self
                .build_collect_asset_fixed_fee(
                    live_acps,
                    live_acps_len - account_number,
                    extra_ckb,
                    estimate_fee,
                )
                .await?;

            Ok(Some(TransactionCompletionResponse::new(
                res.0.into(),
                res.1,
            )))
        }
    }

    fn build_transaction_view(
        &self,
        inputs: &[DetailedCell],
        outputs: Vec<packed::CellOutput>,
        output_data: Vec<packed::Bytes>,
        script_set: HashSet<String>,
    ) -> TransactionView {
        let since: packed::Uint64 = 0u64.pack();
        let deps = script_set
            .iter()
            .map(|s| self.builtin_scripts.get(s).cloned().unwrap().cell_dep)
            .collect::<Vec<_>>();

        TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .cell_deps(deps)
            .inputs(inputs.iter().map(|input| {
                packed::CellInputBuilder::default()
                    .since(since.clone())
                    .previous_output(input.out_point.clone())
                    .build()
            }))
            .outputs(outputs)
            .outputs_data(output_data)
            .build()
    }

    async fn build_create_acp_transaction_fixed_fee(
        &self,
        from: Vec<Item>,
        acp_need_count: usize,
        sudt_type_script: packed::Script,
        item: Item,
        extra_ckb: u64,
        fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureEntry>, AddressPayload)> {
        let mut ckb_needs = fee + MIN_CKB_CAPACITY + extra_ckb;
        let mut outputs_data: Vec<packed::Bytes> = Vec::new();
        let mut outputs = Vec::new();
        let mut change_address = None;

        for i in 0..acp_need_count {
            let capacity = STANDARD_SUDT_CAPACITY + ckb(extra_ckb);
            let output_cell = self.build_acp_cell(
                Some(sudt_type_script.clone()),
                self.get_secp_lock_hash_by_item(item.clone())?.0.to_vec(),
                capacity,
            );

            if i == 0 {
                change_address = Some(AddressPayload::from_script(
                    &output_cell.lock(),
                    self.network_type,
                ));
            }

            outputs.push(output_cell);
            outputs_data.push(Bytes::new().pack());
            ckb_needs += capacity;
        }

        let mut inputs = Vec::new();
        let mut input_capacity_sum = 0u64;
        let mut script_set = HashSet::new();
        let mut signature_entries = HashMap::new();

        if from.is_empty() {
            self.pool_live_cells_by_items(
                vec![item],
                ckb_needs,
                vec![],
                None,
                &mut input_capacity_sum,
                &mut inputs,
                &mut script_set,
                &mut signature_entries,
            )
            .await?;
        } else {
            self.pool_live_cells_by_items(
                from,
                ckb_needs,
                vec![],
                None,
                &mut input_capacity_sum,
                &mut inputs,
                &mut script_set,
                &mut signature_entries,
            )
            .await?;
        }

        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());

        let tx_view = self.build_transaction_view(&inputs, outputs, outputs_data, script_set);

        let mut sigs_entry = signature_entries
            .into_iter()
            .map(|(_k, v)| v)
            .collect::<Vec<_>>();
        sigs_entry.sort();

        Ok((tx_view, sigs_entry, change_address.unwrap()))
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

    async fn build_collect_asset_fixed_fee(
        &self,
        mut acp_cells: Vec<DetailedCell>,
        acp_consume_count: usize,
        extra_ckb: u64,
        fee_rate: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureEntry>)> {
        let acp_need = acp_consume_count + 1;
        let inputs = acp_cells.split_off(acp_need);
        let output = inputs.get(0).cloned().unwrap();
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
            .capacity((input_capacity_sum - extra_ckb).pack())
            .build();
        let output_data = Bytes::from(input_udt_sum.to_le_bytes().to_vec());

        let mut script_set = HashSet::new();
        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());
        script_set.insert(ACP.to_string());

        let sig_entry = SignatureEntry {
            type_: WitnessType::WitnessLock,
            group_len: inputs.len(),
            pub_key: Address::new(
                self.network_type,
                AddressPayload::from_pubkey_hash(self.network_type, pub_key),
            )
            .to_string(),
            signature_type: SignatureType::Secp256k1,
            index: 0,
        };

        let tx_view = self.build_transaction_view(
            &inputs,
            vec![output.clone()],
            vec![output_data.pack()],
            script_set,
        );

        let tx_size = calculate_tx_size_with_witness_placeholder(
            tx_view.clone().into(),
            vec![sig_entry.clone()],
        );
        let actual_fee = fee_rate.saturating_mul(tx_size as u64) * BYTE_SHANNONS / 1000;

        let current_capacity: u64 = output.capacity().unpack();
        let output = output
            .as_builder()
            .capacity((current_capacity - actual_fee).pack())
            .build();
        let tx_view = tx_view.as_advanced_builder().output(output).build();

        Ok((tx_view, vec![sig_entry]))
    }
}

fn parse_from(from_set: HashSet<JsonItem>) -> InnerResult<Vec<Item>> {
    let mut ret: Vec<Item> = Vec::new();
    for ji in from_set.into_iter() {
        ret.push(ji.try_into()?);
    }

    Ok(ret)
}
