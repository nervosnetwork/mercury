use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::utils::address_to_identity;
use crate::rpc_impl::{
    address_to_script, utils, ACP_CODE_HASH, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, CHEQUE_CODE_HASH,
    CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM,
    MIN_CKB_CAPACITY, MIN_DAO_CAPACITY, STANDARD_SUDT_CAPACITY,
};
use crate::types::{
    AddressOrLockHash, AssetInfo, AssetType, ClaimDaoPayload, DaoInfo, DepositPayload, ExtraFilter,
    From, GetBalancePayload, Item, JsonItem, Mode, RequiredUDT, SignatureEntry, SinceConfig,
    SinceFlag, SinceType, SmartTransferPayload, Source, To, ToInfo, TransactionCompletionResponse,
    TransferPayload, UDTInfo, WithdrawPayload,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::hash::blake2b_256_to_160;
use common::utils::{decode_udt_amount, encode_udt_amount};
use common::{Address, DetailedCell, ACP, CHEQUE, DAO, SUDT};
use core_storage::Storage;

use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{ScriptHashType, TransactionBuilder};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256, U256};
use num_traits::Zero;

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::str::FromStr;
use std::vec;

#[derive(Default, Clone, Debug)]
pub struct CellWithData {
    pub cell: packed::CellOutput,
    pub data: packed::Bytes,
}

impl<C: CkbRpc> MercuryRpcImpl<C> {
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
        if payload.amount < MIN_DAO_CAPACITY {
            return Err(RpcErrorMessage::InvalidDAOCapacity);
        }

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let (response, change_cell_index) = self
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
                let tx_view = self.update_tx_view_change_cell_by_index(
                    response.tx_view,
                    change_cell_index,
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
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let mut inputs = Vec::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut script_set = HashSet::new();
        let mut signature_entries = HashMap::new();
        let mut input_index = 0;

        // pool
        let mut items = vec![];
        for json_item in payload.from.items.clone() {
            let item = Item::try_from(json_item)?;
            items.push(item)
        }
        let change_fee_cell_index = self
            .build_required_ckb_and_change_tx_part(
                items.clone(),
                Some(payload.from.source),
                payload.amount + fixed_fee,
                None,
                None,
                &mut inputs,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
                &mut input_index,
            )
            .await?;

        // build deposit cell
        let deposit_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(RpcErrorMessage::InvalidRpcParams(error)),
            },
            None => self.get_secp_address_by_item(items[0].clone())?,
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
        outputs.push(output_deposit);
        cells_data.push(output_data_deposit);

        // build resp
        let inputs = self.build_tx_cell_inputs(&inputs, None, Source::Free)?;
        script_set.insert(DAO.to_string());
        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
        )
        .map(|resp| (resp, change_fee_cell_index))
    }

    pub(crate) async fn inner_build_withdraw_transaction(
        &self,
        payload: WithdrawPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        let item = Item::try_from(payload.clone().from)?;
        let pay_item = match payload.clone().pay_fee {
            Some(pay_fee) => Item::Address(pay_fee),
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
                    response.tx_view.clone(),
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
        let mut input_index = 0;

        self.pool_live_cells_by_items(
            vec![pay_item.clone()],
            MIN_CKB_CAPACITY + estimate_fee,
            vec![],
            None,
            &mut 0,
            &mut input_cells,
            &mut script_set,
            &mut signature_entries,
            &mut input_index,
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
                false,
            )
            .await?;

        let tip_epoch_number: U256 = (**CURRENT_EPOCH_NUMBER.load()).clone().into_u256();
        let deposit_cells = cells
            .into_iter()
            .filter(|cell| cell.cell_data == Box::new([0u8; 8]).to_vec())
            .filter(|cell| cell.epoch_number.clone() + U256::from(4u64) < tip_epoch_number)
            .collect::<Vec<_>>();
        if deposit_cells.is_empty() {
            return Err(RpcErrorMessage::CannotFindDepositCell);
        }

        // build header_deps
        let mut header_deps = HashSet::new();
        for cell in &deposit_cells {
            header_deps.insert(cell.block_hash.pack());
        }
        let header_deps: Vec<packed::Byte32> = header_deps.into_iter().collect();

        // build inputs
        input_cells.extend_from_slice(&deposit_cells);
        let inputs = self.build_tx_cell_inputs(&input_cells, None, Source::Free)?;

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
        let (mut outputs, mut cells_data) = (vec![output_change], vec![Default::default()]);
        outputs.append(&mut outputs_withdraw);
        cells_data.append(&mut outputs_data_withdraw);

        // add signatures
        let cell_sigs: Vec<&SignatureEntry> = signature_entries.iter().map(|(_, s)| s).collect();
        let mut last_index = cell_sigs[0].index; // ensure there is only one sig of pee fee cell
        let address = self.get_secp_address_by_item(item)?;
        for cell in deposit_cells {
            let lock_hash = cell.cell_output.calc_lock_hash().to_string();
            last_index += 1;
            utils::add_sig_entry(
                address.to_string(),
                lock_hash,
                &mut signature_entries,
                last_index,
            );
        }

        // build resp
        script_set.insert(DAO.to_string());

        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            header_deps,
            signature_entries,
        )
    }

    pub(crate) async fn inner_build_claim_dao_transaction(
        &self,
        payload: ClaimDaoPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let (response, change_fee_cell_index) = self
                .prebuild_claim_dao_transaction(payload.clone(), estimate_fee)
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
                let tx_view = self.update_tx_view_change_cell_by_index(
                    response.tx_view,
                    change_fee_cell_index,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.signature_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn prebuild_claim_dao_transaction(
        &self,
        payload: ClaimDaoPayload,
        _fix_fee: u64,
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let item = Item::try_from(payload.clone().from)?;

        // get withdrawing cells as inputs
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        let cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                None,
                Some(ExtraFilter::Dao(DaoInfo::new_withdraw(
                    0,
                    **CURRENT_BLOCK_NUMBER.load(),
                    0,
                ))),
                false,
            )
            .await?;
        let tip_epoch_number: U256 = (**CURRENT_EPOCH_NUMBER.load()).clone().into_u256();
        let withdrawing_cells = cells
            .into_iter()
            .filter(|cell| {
                cell.cell_data != Box::new([0u8; 8]).to_vec() && cell.cell_data.len() == 8
            })
            .filter(|cell| cell.epoch_number.clone() + U256::from(4u64) < tip_epoch_number)
            .collect::<Vec<_>>();
        if withdrawing_cells.is_empty() {
            return Err(RpcErrorMessage::CannotFindWithdrawingCell);
        }

        // build header deps
        let mut header_deps = vec![];
        let mut header_dep_map = HashMap::new();
        let mut deposit_header_dep_indexes: Vec<usize> = vec![];
        for withdrawing_cell in &withdrawing_cells {
            let withdrawing_tx = self
                .inner_get_transaction_with_status(withdrawing_cell.out_point.tx_hash().unpack())
                .await?;
            let input_index: u32 = withdrawing_cell.out_point.index().unpack(); // input deposite cell has the same index
            let deposit_cell = &withdrawing_tx.input_cells[input_index as usize];
            let deposit_block_hash = deposit_cell.block_hash.pack();
            let withdrawing_block_hash = withdrawing_cell.block_hash.pack();
            if !header_dep_map.contains_key(&deposit_block_hash) {
                header_dep_map.insert(deposit_block_hash.clone(), header_deps.len());
                header_deps.push(deposit_block_hash.clone());
            }
            deposit_header_dep_indexes.push(
                header_dep_map
                    .get(&deposit_block_hash)
                    .expect("impossible: get header dep index failed")
                    .to_owned(),
            );
            if !header_dep_map.contains_key(&withdrawing_block_hash) {
                header_dep_map.insert(withdrawing_block_hash.clone(), header_deps.len());
                header_deps.push(withdrawing_block_hash);
            }
        }

        // set since
        

        // calculate award

        // build output cell

        todo!()
    }

    pub(crate) async fn inner_build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.items.is_empty() || payload.to.to_infos.is_empty() {
            return Err(RpcErrorMessage::NeedAtLeastOneFromAndOneTo);
        }
        if payload.from.items.len() > MAX_ITEM_NUM || payload.to.to_infos.len() > MAX_ITEM_NUM {
            return Err(RpcErrorMessage::ExceedMaxItemNum);
        }
        for to_info in &payload.to.to_infos {
            match u128::from_str(&to_info.amount) {
                Ok(amount) => {
                    if amount == 0u128 {
                        return Err(RpcErrorMessage::TransferAmountMustPositive);
                    }
                }
                Err(_) => {
                    return Err(RpcErrorMessage::InvalidRpcParams(
                        "To amount should be a valid u128 number".to_string(),
                    ));
                }
            }
        }

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let (response, change_fee_cell_index) = self
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
                let tx_view = self.update_tx_view_change_cell_by_index(
                    response.tx_view,
                    change_fee_cell_index,
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
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
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
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
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
                    &mut input_index,
                )
                .await?;
        }

        // tx part II
        let mut inputs_part_2 = vec![];
        let mut required_ckb_part_2 = 0;

        // build the outputs
        for to in &payload.to.to_infos {
            let capacity = to
                .amount
                .parse::<u64>()
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            if capacity < MIN_CKB_CAPACITY {
                return Err(RpcErrorMessage::RequiredCKBLessThanMin);
            }
            let item = Item::Address(to.address.to_owned());
            let secp_address = self.get_secp_address_by_item(item)?;
            required_ckb_part_2 += capacity;
            self.build_cell_for_output(
                capacity,
                secp_address.payload().into(),
                None,
                None,
                &mut outputs,
                &mut cells_data,
            )?;
        }

        // build the inputs and the change cell
        let mut items = vec![];
        for json_item in &payload.from.items {
            let item = Item::try_from(json_item.to_owned())?;
            items.push(item)
        }
        if required_ckb_part_1.is_zero() {
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    items,
                    Some(payload.from.source.clone()),
                    required_ckb_part_2 + fixed_fee,
                    payload.change,
                    None,
                    &mut inputs_part_2,
                    &mut script_set,
                    &mut signature_entries,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                items,
                Some(payload.from.source.clone()),
                required_ckb_part_2,
                payload.change,
                None,
                &mut inputs_part_2,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
                &mut input_index,
            )
            .await?;
        };

        // build resp
        let mut inputs = vec![];
        inputs.append(&mut inputs_part_1);
        inputs.append(&mut inputs_part_2);
        let inputs =
            self.build_tx_cell_inputs(&inputs, payload.since.clone(), payload.from.source.clone())?;
        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
        )
        .map(|resp| (resp, change_fee_cell_index))
    }

    async fn prebuild_acp_transfer_transaction_with_ckb(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
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
                    &mut input_index,
                )
                .await?;
        }

        // tx part II: build acp inputs and outputs
        let mut required_ckb_part_2 = 0;
        let mut inputs_part_2 = vec![];

        for to in &payload.to.to_infos {
            let item = Item::Identity(address_to_identity(&to.address)?);

            // build acp input
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
                    false,
                )
                .await?;
            if live_acps.is_empty() {
                return Err(RpcErrorMessage::CannotFindACPCell);
            }

            let current_capacity: u64 = live_acps[0].cell_output.capacity().unpack();
            inputs_part_2.push(live_acps[0].clone());
            input_index += 1;

            // build acp output
            let required_capacity = to
                .amount
                .parse::<u64>()
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            self.build_cell_for_output(
                current_capacity + required_capacity,
                live_acps[0].cell_output.lock(),
                live_acps[0].cell_output.type_().to_opt(),
                None,
                &mut outputs,
                &mut cells_data,
            )?;

            script_set.insert(ACP.to_string());

            required_ckb_part_2 += required_capacity;
        }

        // tx part III:
        let mut from_items = vec![];
        for json_item in payload.from.items {
            let item = Item::try_from(json_item)?;
            from_items.push(item)
        }
        let mut inputs_part_3 = vec![];
        if required_ckb_part_1.is_zero() {
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    from_items,
                    Some(payload.from.source.clone()),
                    required_ckb_part_2 + fixed_fee,
                    payload.change,
                    None,
                    &mut inputs_part_3,
                    &mut script_set,
                    &mut signature_entries,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                from_items,
                Some(payload.from.source.clone()),
                required_ckb_part_2,
                payload.change,
                None,
                &mut inputs_part_3,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
                &mut input_index,
            )
            .await?;
        };

        // build resp
        let mut inputs = vec![];
        inputs.append(&mut inputs_part_1);
        inputs.append(&mut inputs_part_2);
        inputs.append(&mut inputs_part_3);
        let inputs =
            self.build_tx_cell_inputs(&inputs, payload.since.clone(), payload.from.source.clone())?;
        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
        )
        .map(|resp| (resp, change_fee_cell_index))
    }

    async fn prebuild_cheque_transfer_transaction(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;
        script_set.insert(SUDT.to_string());

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
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
                    &mut input_index,
                )
                .await?;
        }

        // tx part II: build cheque outputs
        let mut inputs_part_2: Vec<DetailedCell> = vec![];
        let mut required_udt = 0;
        let mut required_ckb_part_2 = 0;

        for to in &payload.to.to_infos {
            let receiver_address =
                Address::from_str(&to.address).map_err(RpcErrorMessage::InvalidRpcParams)?;
            if !receiver_address.is_secp256k1() {
                return Err(RpcErrorMessage::InvalidRpcParams(
                    "Every to address should be secp/256k1 address".to_string(),
                ));
            }

            // build cheque output
            let sudt_type_script = self
                .build_sudt_type_script(blake2b_256_to_160(&payload.asset_info.udt_hash))
                .await?;
            let to_udt_amount = to
                .amount
                .parse::<u128>()
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            let sender_address = {
                let json_item = &payload.from.items[0];
                let item = Item::try_from(json_item.to_owned())?;
                self.get_secp_address_by_item(item)?
            };
            let cheque_args = utils::build_cheque_args(receiver_address, sender_address);
            let cheque_lock = self
                .get_script_builder(CHEQUE)?
                .args(cheque_args)
                .hash_type(ScriptHashType::Type.into())
                .build();
            self.build_cell_for_output(
                CHEQUE_CELL_CAPACITY,
                cheque_lock,
                Some(sudt_type_script),
                Some(to_udt_amount),
                &mut outputs,
                &mut cells_data,
            )?;
            script_set.insert(CHEQUE.to_string());

            required_udt += to_udt_amount;
            required_ckb_part_2 += CHEQUE_CELL_CAPACITY;
        }

        // tx_part III: pool udt
        let mut pool_udt_amount: u128 = 0;
        let mut inputs_part_3 = vec![];
        let mut from_items = vec![];
        for json_item in payload.from.items {
            let item = Item::try_from(json_item)?;
            from_items.push(item)
        }
        self.build_required_udt_tx_part(
            from_items.clone(),
            Some(payload.from.source.clone()),
            payload.asset_info.udt_hash.clone(),
            required_udt,
            &mut pool_udt_amount,
            &mut inputs_part_3,
            &mut script_set,
            &mut signature_entries,
            &mut outputs,
            &mut cells_data,
            &mut input_index,
        )
        .await?;

        // tx_part IV: pool ckb
        let mut inputs_part_4 = vec![];
        if required_ckb_part_1.is_zero() {
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    from_items,
                    Some(payload.from.source.clone()),
                    required_ckb_part_2 + fixed_fee,
                    payload.change,
                    Some(UDTInfo {
                        asset_info: payload.asset_info,
                        amount: pool_udt_amount - required_udt,
                    }),
                    &mut inputs_part_4,
                    &mut script_set,
                    &mut signature_entries,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                from_items,
                Some(payload.from.source.clone()),
                required_ckb_part_2,
                payload.change,
                Some(UDTInfo {
                    asset_info: payload.asset_info,
                    amount: pool_udt_amount - required_udt,
                }),
                &mut inputs_part_4,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
                &mut input_index,
            )
            .await?;
        };

        // build resp
        let mut inputs = vec![];
        inputs.append(&mut inputs_part_1);
        inputs.append(&mut inputs_part_2);
        inputs.append(&mut inputs_part_3);
        inputs.append(&mut inputs_part_4);
        let inputs =
            self.build_tx_cell_inputs(&inputs, payload.since.clone(), payload.from.source.clone())?;
        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
        )
        .map(|resp| (resp, change_fee_cell_index))
    }

    async fn prebuild_acp_transfer_transaction_with_udt(
        &self,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_entries: HashMap<String, SignatureEntry> = HashMap::new();
        let mut change_fee_cell_index = 0;
        let mut input_index = 0;
        script_set.insert(SUDT.to_string());

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
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
                    &mut input_index,
                )
                .await?;
        }

        // tx part II: build acp inputs and outputs
        let mut required_udt = 0;
        let mut inputs_part_2 = vec![];

        for to in &payload.to.to_infos {
            let item = Item::Identity(address_to_identity(&to.address)?);

            // build acp input
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
                    false,
                )
                .await?;
            if live_acps.is_empty() {
                return Err(RpcErrorMessage::CannotFindACPCell);
            }
            let existing_udt_amount = decode_udt_amount(&live_acps[0].cell_data);
            inputs_part_2.push(live_acps[0].clone());
            input_index += 1;
            script_set.insert(ACP.to_string());

            // build acp output
            let to_udt_amount = to
                .amount
                .parse::<u128>()
                .map_err(|err| RpcErrorMessage::InvalidRpcParams(err.to_string()))?;
            self.build_cell_for_output(
                live_acps[0].cell_output.capacity().unpack(),
                live_acps[0].cell_output.lock(),
                live_acps[0].cell_output.type_().to_opt(),
                Some(existing_udt_amount + to_udt_amount),
                &mut outputs,
                &mut cells_data,
            )?;

            required_udt += to_udt_amount;
        }

        // tx part III: pool udt
        let mut pool_udt_amount: u128 = 0;
        let mut inputs_part_3 = vec![];
        let mut from_items = vec![];
        for json_item in payload.from.items {
            let item = Item::try_from(json_item)?;
            from_items.push(item)
        }
        self.pool_live_cells_by_items(
            from_items.clone(),
            0,
            vec![RequiredUDT {
                udt_hash: payload.asset_info.udt_hash.clone(),
                amount_required: required_udt as i128,
            }],
            Some(payload.from.source.clone()),
            &mut 0,
            &mut inputs_part_3,
            &mut script_set,
            &mut signature_entries,
            &mut input_index,
        )
        .await?;
        for cell in &inputs_part_3 {
            let udt_amount = decode_udt_amount(&cell.cell_data);
            pool_udt_amount += udt_amount;

            let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            if code_hash == **CHEQUE_CODE_HASH.load() {
                let address = match self.generate_ckb_address_or_lock_hash(cell).await? {
                    AddressOrLockHash::Address(address) => address,
                    AddressOrLockHash::LockHash(_) => {
                        return Err(RpcErrorMessage::CannotFindAddressByH160)
                    }
                };
                let address =
                    Address::from_str(&address).map_err(RpcErrorMessage::InvalidRpcParams)?;
                let lock = address_to_script(address.payload());
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    lock,
                    None,
                    None,
                    &mut outputs,
                    &mut cells_data,
                )?;
            } else if code_hash == **ACP_CODE_HASH.load() {
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    cell.cell_output.lock(),
                    cell.cell_output.type_().to_opt(),
                    Some(0),
                    &mut outputs,
                    &mut cells_data,
                )?;
            } else {
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    cell.cell_output.lock(),
                    None,
                    None,
                    &mut outputs,
                    &mut cells_data,
                )?;
            }
        }

        // tx part IV:
        // pool ckb for fee(if needed)
        // and build change cell(both for ckb and udt)
        // if pooling ckb fails, an error will be returned,
        // ckb from the udt cell will no longer be collected
        let mut inputs_part_4 = vec![];
        if required_ckb_part_1.is_zero() {
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    from_items,
                    Some(payload.from.source.clone()),
                    fixed_fee,
                    payload.change,
                    Some(UDTInfo {
                        asset_info: payload.asset_info,
                        amount: pool_udt_amount - required_udt,
                    }),
                    &mut inputs_part_4,
                    &mut script_set,
                    &mut signature_entries,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                from_items,
                Some(payload.from.source.clone()),
                0,
                payload.change,
                Some(UDTInfo {
                    asset_info: payload.asset_info,
                    amount: pool_udt_amount - required_udt,
                }),
                &mut inputs_part_4,
                &mut script_set,
                &mut signature_entries,
                &mut outputs,
                &mut cells_data,
                &mut input_index,
            )
            .await?;
        };

        // build tx
        let mut inputs = vec![];
        inputs.append(&mut inputs_part_1);
        inputs.append(&mut inputs_part_2);
        inputs.append(&mut inputs_part_3);
        inputs.append(&mut inputs_part_4);
        let inputs =
            self.build_tx_cell_inputs(&inputs, payload.since.clone(), payload.from.source.clone())?;
        self.build_tx_complete_resp(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
        )
        .map(|resp| (resp, change_fee_cell_index))
    }

    pub(crate) async fn inner_build_smart_transfer_transaction(
        &self,
        payload: SmartTransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        if payload.from.is_empty() || payload.to.is_empty() {
            return Err(RpcErrorMessage::NeedAtLeastOneFromAndOneTo);
        }
        if payload.from.len() > MAX_ITEM_NUM || payload.to.len() > MAX_ITEM_NUM {
            return Err(RpcErrorMessage::ExceedMaxItemNum);
        }

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);
        loop {
            let (response, change_fee_cell_index) = self
                .prebuild_smart_transfer_transaction(payload.clone(), estimate_fee)
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
                let tx_view = self.update_tx_view_change_cell_by_index(
                    response.tx_view,
                    change_fee_cell_index,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.signature_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn prebuild_smart_transfer_transaction(
        &self,
        payload: SmartTransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionCompletionResponse, usize)> {
        if payload.from.is_empty() || payload.to.is_empty() {
            return Err(RpcErrorMessage::NeedAtLeastOneFromAndOneTo);
        }
        if payload.from.len() > MAX_ITEM_NUM || payload.to.len() > MAX_ITEM_NUM {
            return Err(RpcErrorMessage::ExceedMaxItemNum);
        }

        let mut from_items = vec![];
        for address in &payload.from {
            let identity = address_to_identity(address)?;
            from_items.push(JsonItem::Identity(identity.encode()));
        }
        let mut to_items = vec![];
        for ToInfo { address, .. } in &payload.to {
            let identity = address_to_identity(address)?;
            to_items.push(Item::Identity(identity));
        }
        match payload.asset_info.asset_type {
            AssetType::CKB => {
                let transfer_payload = TransferPayload {
                    asset_info: payload.asset_info,
                    from: From {
                        items: from_items,
                        source: Source::Free,
                    },
                    to: To {
                        to_infos: payload.to,
                        mode: Mode::HoldByFrom,
                    },
                    pay_fee: None,
                    change: payload.change,
                    fee_rate: payload.fee_rate,
                    since: payload.since,
                };
                self.prebuild_secp_transfer_transaction(transfer_payload, fixed_fee)
                    .await
            }
            AssetType::UDT => {
                let mut asset_infos = HashSet::new();
                asset_infos.insert(payload.asset_info.clone());
                let mode = self
                    .get_smart_transfer_mode(&to_items, asset_infos.clone())
                    .await?;
                let source = self
                    .get_smart_transfer_source(&from_items, &payload.to, asset_infos)
                    .await?;
                let mut transfer_payload = TransferPayload {
                    asset_info: payload.asset_info,
                    from: From {
                        items: from_items,
                        source: source.clone(),
                    },
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
                        self.prebuild_cheque_transfer_transaction(transfer_payload, fixed_fee)
                            .await
                    }
                    Mode::HoldByTo => {
                        if Source::Claimable == source {
                            transfer_payload.pay_fee = Some(payload.to[0].address.clone());
                        }
                        self.prebuild_acp_transfer_transaction_with_udt(transfer_payload, fixed_fee)
                            .await
                    }
                }
            }
        }
    }

    async fn get_smart_transfer_mode(
        &self,
        to_items: &[Item],
        asset_infos: HashSet<AssetInfo>,
    ) -> InnerResult<Mode> {
        for i in to_items {
            let live_acps = self
                .get_live_cells_by_item(
                    i.to_owned(),
                    asset_infos.clone(),
                    None,
                    None,
                    Some((**ACP_CODE_HASH.load()).clone()),
                    None,
                    false,
                )
                .await?;
            if live_acps.is_empty() {
                return Ok(Mode::HoldByFrom);
            }
        }
        Ok(Mode::HoldByTo)
    }

    async fn get_smart_transfer_source(
        &self,
        from_items: &[JsonItem],
        to_infos: &[ToInfo],
        asset_infos: HashSet<AssetInfo>,
    ) -> InnerResult<Source> {
        let mut claimable_amount = 0u128;
        let mut free_amount = 0u128;
        let mut required_amount = 0u128;
        for from in from_items {
            let payload = GetBalancePayload {
                item: from.to_owned(),
                asset_infos: asset_infos.clone(),
                tip_block_number: None,
            };
            let resp = self.inner_get_balance(payload).await?;
            for b in resp.balances {
                claimable_amount += b
                    .claimable
                    .parse::<u128>()
                    .map_err(|e| RpcErrorMessage::InvalidRpcParams(e.to_string()))?;
                free_amount += b
                    .free
                    .parse::<u128>()
                    .map_err(|e| RpcErrorMessage::InvalidRpcParams(e.to_string()))?;
            }
        }
        for to in to_infos {
            required_amount += to
                .amount
                .parse::<u128>()
                .map_err(|e| RpcErrorMessage::InvalidRpcParams(e.to_string()))?;
        }

        if claimable_amount >= required_amount {
            Ok(Source::Claimable)
        } else if free_amount >= required_amount {
            Ok(Source::Free)
        } else {
            Err(RpcErrorMessage::UDTIsNotEnough)
        }
    }

    fn build_cell_for_output(
        &self,
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

    pub(crate) async fn build_sudt_type_script(
        &self,
        script_hash: H160,
    ) -> InnerResult<packed::Script> {
        let res = self
            .storage
            .get_scripts(vec![script_hash], vec![], None, vec![])
            .await
            .map_err(|err| RpcErrorMessage::DBError(err.to_string()))?
            .get(0)
            .cloned()
            .ok_or(RpcErrorMessage::CannotGetScriptByHash)?;

        Ok(res)
    }

    fn build_tx_complete_resp(
        &self,
        inputs: Vec<packed::CellInput>,
        outputs: Vec<packed::CellOutput>,
        cells_data: Vec<packed::Bytes>,
        script_set: HashSet<String>,
        header_deps: Vec<packed::Byte32>,
        signature_entries: HashMap<String, SignatureEntry>,
    ) -> InnerResult<TransactionCompletionResponse> {
        let cell_deps = self.build_cell_deps(script_set);
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .outputs(outputs)
            .outputs_data(cells_data)
            .inputs(inputs)
            .cell_deps(cell_deps)
            .header_deps(header_deps)
            .build();

        let mut signature_entries: Vec<SignatureEntry> =
            signature_entries.into_iter().map(|(_, s)| s).collect();
        signature_entries.sort_unstable();

        Ok(TransactionCompletionResponse {
            tx_view: tx_view.into(),
            signature_entries,
        })
    }

    // TODO@chengxing: this function seems to be unnecessary, but it is used by DAO withdraw.
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
                    .header_deps(raw_updated_tx.header_deps())
                    .inputs(raw_updated_tx.inputs())
                    .outputs(raw_updated_tx.outputs())
                    .outputs_data(raw_updated_tx.outputs_data())
                    .build();
                return Ok(updated_tx_view.into());
            }
        }

        Err(RpcErrorMessage::CannotFindChangeCell)
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
        let raw_updated_tx = packed::Transaction::from(tx).raw();
        let updated_tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .cell_deps(raw_updated_tx.cell_deps())
            .inputs(raw_updated_tx.inputs())
            .outputs(raw_updated_tx.outputs())
            .outputs_data(raw_updated_tx.outputs_data())
            .build();
        Ok(updated_tx_view.into())
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
        inputs: &[DetailedCell],
        since: Option<SinceConfig>,
        source: Source,
    ) -> InnerResult<Vec<packed::CellInput>> {
        let since = if let Some(config) = since {
            utils::to_since(config)?
        } else {
            0u64
        };
        let inputs: Vec<packed::CellInput> = inputs
            .iter()
            .map(|cell| {
                let since = if source == Source::Free
                    && self.is_script(&cell.cell_output.lock(), CHEQUE).unwrap()
                {
                    // cheque cell since must be hardcoded as 0xA000000000000006
                    let config = SinceConfig {
                        flag: SinceFlag::Relative,
                        type_: SinceType::EpochNumber,
                        value: 6,
                    };
                    utils::to_since(config).unwrap()
                } else {
                    since
                };
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
        udt_change_info: Option<UDTInfo>,
        inputs: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_entries: &mut HashMap<String, SignatureEntry>,
        outputs: &mut Vec<packed::CellOutput>,
        cells_data: &mut Vec<packed::Bytes>,
        input_index: &mut usize,
    ) -> InnerResult<usize> {
        let required_ckb = if let Some(udt_info) = &udt_change_info {
            if udt_info.amount != 0 {
                required_ckb + STANDARD_SUDT_CAPACITY + MIN_CKB_CAPACITY
            } else {
                required_ckb + MIN_CKB_CAPACITY
            }
        } else {
            required_ckb + MIN_CKB_CAPACITY
        };

        self.pool_live_cells_by_items(
            items.to_owned(),
            required_ckb,
            vec![],
            source,
            &mut 0,
            inputs,
            script_set,
            signature_entries,
            input_index,
        )
        .await?;

        // build change cell
        let pool_capacity = get_pool_capacity(inputs)?;
        let item = if let Some(address) = change_address {
            Item::Address(address)
        } else {
            items[0].to_owned()
        };
        let secp_address = self.get_secp_address_by_item(item)?;

        if let Some(udt_info) = udt_change_info {
            if udt_info.amount != 0 {
                let type_script = self
                    .build_sudt_type_script(blake2b_256_to_160(&udt_info.asset_info.udt_hash))
                    .await?;
                self.build_cell_for_output(
                    STANDARD_SUDT_CAPACITY,
                    secp_address.payload().into(),
                    Some(type_script),
                    Some(udt_info.amount),
                    outputs,
                    cells_data,
                )?;
            }
        }

        let change_cell_capacity = pool_capacity - required_ckb + MIN_CKB_CAPACITY;

        let change_cell_index = self.build_cell_for_output(
            change_cell_capacity,
            secp_address.payload().into(),
            None,
            None,
            outputs,
            cells_data,
        )?;
        Ok(change_cell_index)
    }

    async fn build_required_udt_tx_part(
        &self,
        from_items: Vec<Item>,
        source: Option<Source>,
        udt_hash: H256,
        required_udt: u128,
        pool_udt_amount: &mut u128,
        inputs: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_entries: &mut HashMap<String, SignatureEntry>,
        outputs: &mut Vec<packed::CellOutput>,
        cells_data: &mut Vec<packed::Bytes>,
        input_index: &mut usize,
    ) -> InnerResult<()> {
        self.pool_live_cells_by_items(
            from_items.clone(),
            0,
            vec![RequiredUDT {
                udt_hash,
                amount_required: required_udt as i128,
            }],
            source,
            &mut 0,
            inputs,
            script_set,
            signature_entries,
            input_index,
        )
        .await?;
        for cell in inputs {
            let udt_amount = decode_udt_amount(&cell.cell_data);
            *pool_udt_amount += udt_amount;

            let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            if code_hash == **CHEQUE_CODE_HASH.load() {
                let address = match self.generate_ckb_address_or_lock_hash(cell).await? {
                    AddressOrLockHash::Address(address) => address,
                    AddressOrLockHash::LockHash(_) => {
                        return Err(RpcErrorMessage::CannotFindAddressByH160)
                    }
                };
                let address =
                    Address::from_str(&address).map_err(RpcErrorMessage::InvalidRpcParams)?;
                let lock = address_to_script(address.payload());
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    lock,
                    None,
                    None,
                    outputs,
                    cells_data,
                )?;
            } else if code_hash == **ACP_CODE_HASH.load() {
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    cell.cell_output.lock(),
                    cell.cell_output.type_().to_opt(),
                    Some(0),
                    outputs,
                    cells_data,
                )?;
            } else {
                self.build_cell_for_output(
                    cell.cell_output.capacity().unpack(),
                    cell.cell_output.lock(),
                    None,
                    None,
                    outputs,
                    cells_data,
                )?;
            }
        }
        Ok(())
    }
}

fn get_pool_capacity(inputs: &[DetailedCell]) -> InnerResult<u64> {
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
        .header_deps(raw_tx.header_deps())
        .inputs(raw_tx.inputs())
        .outputs(raw_tx.outputs())
        .outputs_data(raw_tx.outputs_data())
        .witnesses(witnesses)
        .build();
    let tx_size = tx_view_with_witness_placeholder.data().total_size();
    // tx offset bytesize
    tx_size + 4
}
