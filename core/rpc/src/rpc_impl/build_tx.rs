use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, utils, ACP_CODE_HASH, BYTE_SHANNONS, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER,
    CURRENT_EPOCH_NUMBER, DAO_CODE_HASH, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM,
    MIN_CKB_CAPACITY, SECP256K1_CODE_HASH, STANDARD_SUDT_CAPACITY, SUDT_CODE_HASH,
};
use crate::types::{
    decode_record_id, encode_record_id, AdjustAccountPayload, AssetInfo, AssetType, DaoInfo,
    DaoState, DepositPayload, ExtraFilter, From, IOType, Identity, IdentityFlag, Item, JsonItem,
    Mode, Record, RequiredUDT, SignatureEntry, SignatureType, SinceConfig, Source, Status, To,
    ToInfo, TransactionCompletionResponse, TransferPayload, UDTInfo, WithdrawPayload, WitnessType,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_dao_block_number, decode_udt_amount, encode_udt_amount, parse_address};
use common::{
    Address, AddressPayload, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1, SUDT,
};
use core_storage::Storage;

use ckb_jsonrpc_types::{CellOutput, TransactionView as JsonTransactionView};
use ckb_types::core::{
    BlockNumber, EpochNumberWithFraction, RationalU256, ScriptHashType, TransactionBuilder,
    TransactionView,
};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256, U256};
use num_bigint::BigInt;
use num_traits::Zero;

use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;
use std::str::FromStr;

const fn ckb(num: u64) -> u64 {
    num * BYTE_SHANNONS
}

#[derive(Default, Clone, Debug)]
pub struct CellWithData {
    pub cell: packed::CellOutput,
    pub data: packed::Bytes,
}

impl CellWithData {
    pub fn new(cell: packed::CellOutput, data: Bytes) -> Self {
        CellWithData {
            cell,
            data: data.pack(),
        }
    }
}

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> InnerResult<Option<TransactionCompletionResponse>> {
        if payload.asset_info.asset_type == AssetType::CKB {
            return Err(RpcErrorMessage::AdjustAccountOnCkb);
        }

        let account_number = payload.account_number.unwrap_or(1) as usize;
        let extra_ckb = payload.extra_ckb.unwrap_or(1);
        let fee_rate = payload.fee_rate.unwrap_or(1000);

        let item: Item = payload.item.clone().try_into()?;
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

        let sudt_type_script = self.build_sudt_type_script(payload.asset_info.udt_hash.0.to_vec());
        let from = parse_from(payload.from.clone())?;

        if live_acps_len < account_number {
            self.build_create_acp_transaction(
                from,
                account_number - live_acps_len,
                sudt_type_script,
                item,
                extra_ckb,
                fee_rate,
            )
            .await?;
        } else {
        }

        todo!()
    }

    pub(crate) async fn inner_build_deposit_transaction(
        &self,
        payload: DepositPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.items.is_empty() {
            return Err(RpcErrorMessage::NeedAtLeastOneFrom);
        }
        if payload.from.items.len() > MAX_ITEM_NUM {
            return Err(RpcErrorMessage::ExceedMaxItemNum);
        }

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let response = self
                .prebuild_deposit_transaction(payload.clone(), estimate_fee)
                .await?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.signature_entries.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let item = payload.from.items[0].clone().try_into()?;
                let change_address = self.get_secp_address_by_item(item)?;
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    change_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.signature_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn prebuild_deposit_transaction(
        &self,
        payload: DepositPayload,
        fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        let json_items: Vec<JsonItem> = payload.from.items.clone();
        let mut items = vec![];
        for json_item in json_items {
            let item = Item::try_from(json_item)?;
            items.push(item)
        }

        // pool
        let mut inputs = Vec::new();
        let mut script_set = HashSet::new();
        let mut signature_entries = HashMap::new();
        self.pool_live_cells_by_items(
            items.clone(),
            (payload.amount + MIN_CKB_CAPACITY + fixed_fee) as i64,
            vec![],
            Some(payload.from.source),
            &mut inputs,
            &mut script_set,
            &mut signature_entries,
        )
        .await?;

        // build change cell
        let pool_capacity = get_pool_capacity(&inputs)?;
        let change_address = self.get_secp_address_by_item(items[0].clone())?;
        let output_change = packed::CellOutputBuilder::default()
            .capacity((pool_capacity - fixed_fee).pack())
            .lock(change_address.payload().into())
            .build();

        // build deposit cell
        let deposit_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(RpcErrorMessage::InvalidRpcParams(error)),
            },
            None => self.get_secp_address_by_item(items[0].clone())?,
        };
        let type_script = self
            .get_script_builder(DAO)
            .hash_type(ScriptHashType::Type.into())
            .build();
        let output_deposit = packed::CellOutputBuilder::default()
            .capacity(payload.amount.pack())
            .lock(deposit_address.payload().into())
            .type_(Some(type_script).pack())
            .build();
        let output_data_deposit: packed::Bytes = Bytes::from(vec![0u8; 8]).pack();

        // build inputs
        let inputs = self.build_tx_cell_inputs(&inputs, None)?;

        // build cell_deps
        script_set.insert(DAO.to_string());
        let cell_deps = self.build_cell_deps(script_set);

        // build tx
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .output(output_deposit)
            .output_data(output_data_deposit)
            .output(output_change)
            .output_data(Default::default())
            .inputs(inputs)
            .cell_deps(cell_deps)
            .build();

        let mut signature_entries: Vec<SignatureEntry> =
            signature_entries.into_iter().map(|(_, s)| s).collect();
        signature_entries.sort_unstable();

        Ok(TransactionCompletionResponse {
            tx_view: tx_view.into(),
            signature_entries,
        })
    }

    pub(crate) async fn inner_build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.items.is_empty()
            || (payload.to.to_infos.is_empty() && payload.change.is_none())
        {
            return Err(RpcErrorMessage::NeedAtLeastOneFromAndOneTo);
        }
        if payload.from.items.len() > MAX_ITEM_NUM || payload.to.to_infos.len() > MAX_ITEM_NUM {
            return Err(RpcErrorMessage::ExceedMaxItemNum);
        }

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let response = self
                .prebuild_transfer_transaction(payload.clone(), estimate_fee)
                .await?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.signature_entries.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let change_item = if let Some(address) = &payload.pay_fee {
                    Item::Address(address.to_owned())
                } else if let Some(address) = &payload.change {
                    Item::Address(address.to_owned())
                } else {
                    Item::try_from(payload.from.items[0].clone())?
                };
                let change_address = self.get_secp_address_by_item(change_item)?;
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    change_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.signature_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn prebuild_transfer_transaction(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        match (&payload.asset_info.asset_type, &payload.to.mode) {
            (AssetType::CKB, Mode::HoldByFrom) => {
                self.prebuild_secp_transfer_transaction(payload, fixed_fee)
                    .await
            }
            (AssetType::CKB, Mode::HoldByTo) => {
                self.prebuild_acp_transfer_transaction_with_ckb(payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::HoldByFrom) => {
                self.prebuild_cheque_transfer_transaction(payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::HoldByTo) => {
                self.prebuild_acp_transfer_transaction_with_udt(payload, fixed_fee)
                    .await
            }
        }
    }

    async fn prebuild_secp_transfer_transaction(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Address(pay_address.to_owned())];
            required_ckb_part_1 += MIN_CKB_CAPACITY + fixed_fee;
            self.build_required_ckb_and_change_tx_part(
                items,
                None,
                required_ckb_part_1,
                None,
                None,
                &mut inputs_part_1,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
            )
            .await?;
        }

        // tx part II: pool ckb and build change
        let mut inputs_part_2 = vec![];
        let mut required_ckb_part_2 = 0;

        // build the outputs
        for to in &payload.to.to_infos {
            let capacity = u64::from_str_radix(&to.amount, 10)
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            if capacity < MIN_CKB_CAPACITY {
                return Err(RpcErrorMessage::RequiredCKBLessThanMin);
            }
            let item = Item::Address(to.address.to_owned());
            self.build_secp_cell_for_output(
                item,
                capacity,
                None,
                None,
                &mut outputs,
                &mut cells_data,
                &mut required_ckb_part_2,
            )?;
        }

        // build the inputs
        required_ckb_part_2 += if required_ckb_part_1.is_zero() {
            MIN_CKB_CAPACITY + fixed_fee
        } else {
            MIN_CKB_CAPACITY
        };
        let mut items = vec![];
        for json_item in &payload.from.items {
            let item = Item::try_from(json_item.to_owned())?;
            items.push(item)
        }
        self.build_required_ckb_and_change_tx_part(
            items,
            Some(payload.from.source),
            required_ckb_part_2,
            payload.change,
            None,
            &mut inputs_part_2,
            &mut script_set,
            &mut signature_entries,
            &mut outputs,
            &mut cells_data,
        )
        .await?;

        // build cell_deps
        let cell_deps = self.build_cell_deps(script_set);

        // build tx
        let mut inputs = vec![];
        inputs.append(&mut inputs_part_1);
        inputs.append(&mut inputs_part_2);
        let inputs = self.build_tx_cell_inputs(&inputs, None)?;
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .outputs(outputs)
            .outputs_data(cells_data)
            .inputs(inputs)
            .cell_deps(cell_deps)
            .build();

        let mut signature_entries: Vec<SignatureEntry> =
            signature_entries.into_iter().map(|(_, s)| s).collect();
        signature_entries.sort_unstable();

        Ok(TransactionCompletionResponse {
            tx_view: tx_view.into(),
            signature_entries,
        })
    }

    async fn prebuild_acp_transfer_transaction_with_ckb(
        &self,
        _payload: TransferPayload,
        _fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        todo!()
    }

    async fn prebuild_cheque_transfer_transaction(
        &self,
        _payload: TransferPayload,
        _fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        todo!()
    }

    async fn prebuild_acp_transfer_transaction_with_udt(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Address(pay_address.to_owned())];
            required_ckb_part_1 += MIN_CKB_CAPACITY + fixed_fee;
            self.build_required_ckb_and_change_tx_part(
                items,
                None,
                required_ckb_part_1,
                None,
                None,
                &mut inputs_part_1,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
            )
            .await?;
        }

        // tx part II: pool udt and build udt change
        // let mut inputs_part_2 = vec![];
        let mut required_udt_part_2 = 0;
        let pool_udt_amount: u128 = 0;

        // build the outputs
        let sudt_type_script = self.build_sudt_type_script(payload.asset_info.udt_hash.0.to_vec());
        for to in &payload.to.to_infos {
            let amount = u128::from_str_radix(&to.amount, 10)
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            let item = Item::Address(to.address.to_owned());
            let _output_cell = self.build_acp_cell(
                Some(sudt_type_script.clone()),
                self.get_secp_lock_hash_by_item(item.clone())?.0.to_vec(),
                STANDARD_SUDT_CAPACITY,
            );
            required_udt_part_2 += amount;
        }

        // tx part III: pool ckb for fee and change cell(both for ckb and udt)
        // if pooling ckb fails, an error will be returned,
        // ckb from the udt cell will no longer be collected
        let mut inputs_part_3 = vec![];
        let required_ckb_part_3 = if required_ckb_part_1.is_zero() {
            STANDARD_SUDT_CAPACITY + fixed_fee
        } else {
            STANDARD_SUDT_CAPACITY
        };
        let mut items = vec![];
        for json_item in &payload.from.items {
            let item = Item::try_from(json_item.to_owned())?;
            items.push(item)
        }
        self.build_required_ckb_and_change_tx_part(
            items,
            Some(payload.from.source),
            required_ckb_part_3,
            payload.change,
            Some(UDTInfo {
                asset_info: payload.asset_info,
                amount: pool_udt_amount - required_udt_part_2,
            }),
            &mut inputs_part_3,
            &mut script_set,
            &mut signature_entries,
            &mut outputs,
            &mut cells_data,
        )
        .await?;

        todo!();
    }

    fn build_secp_cell_for_output(
        &self,
        item: Item,
        capacity: u64,
        type_script: Option<packed::Script>,
        udt_amount: Option<u128>,
        outputs: &mut Vec<packed::CellOutput>,
        cells_data: &mut Vec<packed::Bytes>,
        required_ckb: &mut u64,
    ) -> InnerResult<()> {
        let secp_address = self.get_secp_address_by_item(item)?;
        let cell_output = packed::CellOutputBuilder::default()
            .lock(secp_address.payload().into())
            .type_(type_script.pack())
            .capacity(capacity.pack())
            .build();
        outputs.push(cell_output);

        let data: packed::Bytes = if let Some(udt_amount) = udt_amount {
            Bytes::from(encode_udt_amount(udt_amount)).pack()
        } else {
            Default::default()
        };
        cells_data.push(data);

        *required_ckb += capacity;
        Ok(())
    }

    fn build_cheque_cells_with_data(
        &self,
        _address: &Address,
        _amount: u128,
        _required_ckb: &mut i64,
        _required_udts: &mut Vec<RequiredUDT>,
    ) -> InnerResult<CellWithData> {
        todo!()
    }

    async fn build_create_acp_transaction(
        &self,
        from: Vec<Item>,
        acp_need_count: usize,
        sudt_type_script: packed::Script,
        item: Item,
        extra_ckb: u64,
        _fee_rate: u64,
    ) -> InnerResult<()> {
        let mut ckb_needs = ckb(1);
        let (mut outputs, mut outputs_data) = (vec![], vec![]);
        for _i in 0..acp_need_count {
            let capacity = STANDARD_SUDT_CAPACITY + ckb(extra_ckb);
            let output_cell = self.build_acp_cell(
                Some(sudt_type_script.clone()),
                self.get_secp_lock_hash_by_item(item.clone())?.0.to_vec(),
                capacity,
            );
            outputs.push(output_cell);
            outputs_data.push(Bytes::new());
            ckb_needs += capacity;
        }

        let mut inputs = Vec::new();
        let mut script_set = HashSet::new();
        let mut signature_entries = HashMap::new();

        if from.is_empty() {
            self.pool_live_cells_by_items(
                vec![item],
                ckb_needs as i64,
                vec![],
                None,
                &mut inputs,
                &mut script_set,
                &mut signature_entries,
            )
            .await?;
        } else {
            self.pool_live_cells_by_items(
                from,
                ckb_needs as i64,
                vec![],
                None,
                &mut inputs,
                &mut script_set,
                &mut signature_entries,
            )
            .await?;
        }

        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());

        Ok(())
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

    fn build_sudt_type_script(&self, type_args: Vec<u8>) -> packed::Script {
        self.builtin_scripts
            .get(SUDT)
            .cloned()
            .expect("Impossible: get built in script fail")
            .script
            .as_builder()
            .args(type_args.pack())
            .build()
    }

    fn build_tx_complete_resp(
        &self,
        _fee_rate: u64,
        _inputs: &[DetailedCell],
        _script_set: &mut HashSet<String>,
        _signature_entries: &mut HashMap<String, SignatureEntry>,
    ) -> InnerResult<TransactionCompletionResponse> {
        todo!()
    }

    pub(crate) fn update_tx_view_change_cell(
        &self,
        tx_view: JsonTransactionView,
        change_address: Address,
        estimate_fee: u64,
        actual_fee: u64,
    ) -> InnerResult<JsonTransactionView> {
        let mut tx = tx_view.inner;
        let change_cell_lock = address_to_script(change_address.payload());
        for output in &mut tx.outputs {
            if output.lock == change_cell_lock.clone().into() && output.type_.is_none() {
                let change_cell_capacity: u64 = output.capacity.into();
                let updated_change_cell_capacity = change_cell_capacity + estimate_fee - actual_fee;
                let updated_change_cell = packed::CellOutputBuilder::default()
                    .lock(change_cell_lock)
                    .capacity(updated_change_cell_capacity.pack())
                    .build();
                *output = updated_change_cell.into();
                let raw_updated_tx = packed::Transaction::from(tx).raw();
                let updated_tx_view = TransactionBuilder::default()
                    .version(TX_VERSION.pack())
                    .cell_deps(raw_updated_tx.cell_deps())
                    .inputs(raw_updated_tx.inputs())
                    .outputs(raw_updated_tx.outputs())
                    .outputs_data(raw_updated_tx.outputs_data())
                    .build();
                return Ok(updated_tx_view.into());
            }
        }

        Err(RpcErrorMessage::CannotFindChangeCell)
    }

    pub(crate) async fn inner_build_withdraw_transaction(
        &self,
        payload: WithdrawPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        let item = Item::try_from(payload.clone().from)?;
        let pay_item = match payload.clone().pay_fee {
            Some(pay_item) => Item::try_from(pay_item)?,
            None => item.clone(),
        };

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let response = self
                .prebuild_withdraw_transaction(item.clone(), pay_item.clone(), estimate_fee)
                .await?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.signature_entries.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let change_address = self.get_secp_address_by_item(pay_item)?;
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    change_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.signature_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn prebuild_withdraw_transaction(
        &self,
        item: Item,
        pay_item: Item,
        estimate_fee: u64,
    ) -> InnerResult<TransactionCompletionResponse> {
        // pool ckb for fee
        let mut input_cells = Vec::new();
        let mut script_set = HashSet::new();
        let mut signature_entries = HashMap::new();
        self.pool_live_cells_by_items(
            vec![pay_item.clone()],
            (MIN_CKB_CAPACITY + estimate_fee) as i64,
            vec![],
            None,
            &mut input_cells,
            &mut script_set,
            &mut signature_entries,
        )
        .await?;

        // build output change cell
        let pay_cell_capacity: u64 = input_cells[0].cell_output.capacity().unpack();
        let change_address = self.get_secp_address_by_item(pay_item.clone())?;
        let output_change = packed::CellOutputBuilder::default()
            .capacity((pay_cell_capacity - estimate_fee).pack())
            .lock(change_address.payload().into())
            .build();

        // This check ensures that only one pay fee cell is placed first in the input
        // and the change cell is placed first in the output,
        // so that the index of each input deposit cell
        // and the corresponding withdrawing cell are the same,
        // which meets the withdrawing tx(phase I) requirements
        if input_cells.len() > 1 {
            return Err(RpcErrorMessage::CannotFindChangeCell);
        }

        // get deposit cells
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        let cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                None,
                Some(ExtraFilter::Dao(DaoInfo::new_deposit(0, 0))),
            )
            .await?;
        let mut deposit_cells = cells
            .into_iter()
            .filter(|cell| cell.cell_data == Box::new([0u8; 8]).to_vec())
            .collect::<Vec<_>>();

        // build header_deps
        let tip_block_number = self
            .storage
            .get_tip()
            .await
            .map_err(|err| RpcErrorMessage::DBError(err.to_string()))?
            .unwrap_or((0, H256::default()))
            .0;
        let tip_epoch_number: U256 = self
            .get_epoch_by_number(tip_block_number)
            .await?
            .into_u256();
        let mut header_deps = HashSet::new();
        for cell in &deposit_cells {
            if cell.epoch_number.clone() + U256::from(4u64) > tip_epoch_number {
                return Err(RpcErrorMessage::CannotReferenceHeader);
            }
            let header = self
                .storage
                .get_block_header(Some(cell.block_hash.clone()), Some(cell.block_number))
                .await
                .map_err(|err| RpcErrorMessage::DBError(err.to_string()))?;
            header_deps.insert(header.hash());
        }
        let header_deps: Vec<packed::Byte32> = header_deps.into_iter().collect();

        // build inputs
        input_cells.append(&mut deposit_cells);
        let inputs = self.build_tx_cell_inputs(&input_cells, None)?;

        // build output withdrawing cells
        let outputs_withdraw: Vec<packed::CellOutput> = deposit_cells
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
        let outputs_data_withdraw: Vec<packed::Bytes> = deposit_cells
            .iter()
            .map(|cell| {
                let data: packed::Uint64 = cell.block_number.pack();
                data.as_bytes().pack()
            })
            .collect();

        // build cell_deps
        script_set.insert(DAO.to_string());
        let cell_deps = self.build_cell_deps(script_set);

        // build tx
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .inputs(inputs)
            .output(output_change)
            .output_data(Default::default())
            .outputs(outputs_withdraw)
            .outputs_data(outputs_data_withdraw)
            .cell_deps(cell_deps)
            .header_deps(header_deps)
            .build();

        // add signatures
        let pay_fee_cell_sigs: Vec<&SignatureEntry> =
            signature_entries.iter().map(|(_, s)| s).collect();
        let mut index = pay_fee_cell_sigs[0].index;
        let address = self.get_secp_address_by_item(item)?;
        for cell in deposit_cells {
            let lock_hash = cell.cell_output.calc_lock_hash().to_string();
            index += 1;
            utils::add_sig_entry(
                address.to_string(),
                lock_hash,
                &mut signature_entries,
                index,
            );
        }
        let mut signature_entries: Vec<SignatureEntry> =
            signature_entries.into_iter().map(|(_, s)| s).collect();
        signature_entries.sort_unstable();

        Ok(TransactionCompletionResponse {
            tx_view: tx_view.into(),
            signature_entries,
        })
    }

    fn build_cell_deps(&self, script_set: HashSet<String>) -> Vec<packed::CellDep> {
        script_set
            .into_iter()
            .map(|s| {
                self.builtin_scripts
                    .get(s.as_str())
                    .cloned()
                    .expect("Impossible: get builtin script fail")
                    .cell_dep
            })
            .collect()
    }

    fn build_tx_cell_inputs(
        &self,
        inputs: &Vec<DetailedCell>,
        since: Option<SinceConfig>,
    ) -> InnerResult<Vec<packed::CellInput>> {
        let since = if let Some(_config) = since {
            // todo: since
            0u64
        } else {
            0u64
        };
        let inputs: Vec<packed::CellInput> = inputs
            .iter()
            .map(|cell| {
                packed::CellInputBuilder::default()
                    .since(since.pack())
                    .previous_output(cell.out_point.clone())
                    .build()
            })
            .collect();
        Ok(inputs)
    }

    async fn build_required_ckb_and_change_tx_part(
        &self,
        items: Vec<Item>,
        source: Option<Source>,
        required_ckb: u64,
        change_address: Option<String>,
        udt_info: Option<UDTInfo>,
        inputs: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_entries: &mut HashMap<String, SignatureEntry>,
        outputs: &mut Vec<packed::CellOutput>,
        cells_data: &mut Vec<packed::Bytes>,
    ) -> InnerResult<()> {
        self.pool_live_cells_by_items(
            items.to_owned(),
            required_ckb as i64,
            vec![],
            source,
            inputs,
            script_set,
            signature_entries,
        )
        .await?;

        // build change cell
        let pool_capacity = get_pool_capacity(&inputs)?;
        let (base_capacity, type_script, preset_udt_amount) = if let Some(udt_info) = udt_info {
            (
                STANDARD_SUDT_CAPACITY,
                Some(self.build_sudt_type_script(udt_info.asset_info.udt_hash.0.to_vec())),
                Some(udt_info.amount),
            )
        } else {
            (MIN_CKB_CAPACITY, None, None)
        };
        let change_cell_capacity = pool_capacity - required_ckb + base_capacity;
        let item = if let Some(address) = change_address {
            Item::Address(address)
        } else {
            items[0].to_owned()
        };
        self.build_secp_cell_for_output(
            item,
            change_cell_capacity,
            type_script,
            preset_udt_amount,
            outputs,
            cells_data,
            &mut 0,
        )?;
        Ok(())
    }
}

fn get_pool_capacity(inputs: &Vec<DetailedCell>) -> InnerResult<u64> {
    // todo: add dao reward
    let pool_capacity: u64 = inputs
        .iter()
        .map(|cell| {
            let capacity: u64 = cell.cell_output.capacity().unpack();
            capacity
        })
        .sum();
    Ok(pool_capacity)
}

fn parse_from(from_set: HashSet<JsonItem>) -> InnerResult<Vec<Item>> {
    let mut ret: Vec<Item> = Vec::new();
    for ji in from_set.into_iter() {
        ret.push(ji.try_into()?);
    }

    Ok(ret)
}

pub fn calculate_tx_size_with_witness_placeholder(
    tx_view: JsonTransactionView,
    sigs_entry: Vec<SignatureEntry>,
) -> usize {
    let tx = tx_view.inner;
    let raw_tx = packed::Transaction::from(tx.clone()).raw();
    let mut witnesses_map = HashMap::new();
    for (index, _input) in tx.inputs.into_iter().enumerate() {
        witnesses_map.insert(index, Bytes::new());
    }
    for sig_entry in sigs_entry {
        let witness = packed::WitnessArgs::new_builder()
            .lock(Some(Bytes::from(vec![0u8; 65])).pack())
            .build();
        witnesses_map.insert(sig_entry.index, witness.as_bytes());
    }

    let witnesses: Vec<packed::Bytes> = witnesses_map
        .into_iter()
        .map(|(_index, witness)| witness.pack())
        .collect();

    let tx_view_with_witness_placeholder = TransactionBuilder::default()
        .version(TX_VERSION.pack())
        .cell_deps(raw_tx.cell_deps())
        .inputs(raw_tx.inputs())
        .outputs(raw_tx.outputs())
        .outputs_data(raw_tx.outputs_data())
        .witnesses(witnesses)
        .build();
    let tx_size = tx_view_with_witness_placeholder.data().total_size();
    // tx offset bytesize
    tx_size + 4
}
