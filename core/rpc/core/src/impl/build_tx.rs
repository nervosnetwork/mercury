use crate::r#impl::{address_to_script, utils};
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use common::hash::blake2b_256_to_160;
use common::utils::{decode_udt_amount, encode_udt_amount};
use common::{Address, Context, DetailedCell, ACP, CHEQUE, DAO, SECP256K1, SUDT};
use common_logger::tracing_async;
use core_ckb_client::CkbRpc;
use core_rpc_types::consts::{
    BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM,
    MIN_CKB_CAPACITY, MIN_DAO_CAPACITY, STANDARD_SUDT_CAPACITY,
};
use core_rpc_types::lazy::{
    ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_EPOCH_NUMBER, SECP256K1_CODE_HASH,
};
use core_rpc_types::{
    AddressOrLockHash, AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload,
    DaoWithdrawPayload, ExtraType, From, GetBalancePayload, HashAlgorithm, Item, JsonItem, Mode,
    RequiredUDT, SignAlgorithm, SignatureAction, SimpleTransferPayload, SinceConfig, SinceFlag,
    SinceType, Source, To, ToInfo, TransactionCompletionResponse, TransferPayload, UDTInfo,
};
use core_storage::Storage;

use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{
    EpochNumberWithFraction, ScriptHashType, TransactionBuilder, TransactionView,
};
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
        if payload.amount < MIN_DAO_CAPACITY {
            return Err(CoreError::InvalidDAOCapacity.into());
        }
        utils::check_same_enum_value(payload.from.items.iter().collect())?;
        let mut payload = payload;
        payload.from.items = utils::dedup_json_items(payload.from.items);

        self.build_transaction_with_adjusted_fee(
            Self::prebuild_dao_deposit_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_deposit_transaction(
        &self,
        ctx: Context,
        payload: DaoDepositPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
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
                ctx.clone(),
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

        // build output deposit cell
        let deposit_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(CoreError::InvalidRpcParams(error).into()),
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
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_entries,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
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
            payload.fee_rate,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_withdraw_transaction(
        &self,
        ctx: Context,
        payload: DaoWithdrawPayload,
        estimate_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let item = Item::try_from(payload.clone().from)?;
        let pay_item = match payload.clone().pay_fee {
            Some(pay_fee) => Item::Address(pay_fee),
            None => item.clone(),
        };

        // pool ckb for fee
        let mut input_cells = Vec::new();
        let mut script_set = HashSet::new();
        let mut signature_actions = HashMap::new();
        let mut input_index = 0;

        self.pool_live_cells_by_items(
            ctx.clone(),
            vec![pay_item.clone()],
            MIN_CKB_CAPACITY + estimate_fee,
            vec![],
            None,
            &mut 0,
            &mut input_cells,
            &mut script_set,
            &mut signature_actions,
            &mut input_index,
        )
        .await?;

        // build output change cell
        let change_fee_cell_index = 0;
        let pay_cell_capacity: u64 = input_cells[change_fee_cell_index]
            .cell_output
            .capacity()
            .unpack();
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
            return Err(CoreError::CannotFindChangeCell.into());
        }

        // get deposit cells
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        let cells = self
            .get_live_cells_by_item(
                ctx.clone(),
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                Some((**SECP256K1_CODE_HASH.load()).clone()),
                Some(ExtraType::Dao),
                false,
            )
            .await?;

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
        // let cell_sigs: Vec<&SignatureEntry> = signature_entries.iter().map(|(_, s)| s).collect();
        // let mut last_index = cell_sigs[0].index; // ensure there is only one sig of pay fee cell
        let address = self.get_secp_address_by_item(item)?;
        for cell in deposit_cells {
            let lock_hash = cell.cell_output.calc_lock_hash().to_string();
            utils::add_signature_action(
                address.to_string(),
                lock_hash,
                SignAlgorithm::Secp256k1,
                HashAlgorithm::Blake2b,
                &mut signature_actions,
                input_index,
            );
            input_index += 1;
        }

        // build resp
        script_set.insert(DAO.to_string());

        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            header_deps,
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
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
            payload.fee_rate,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_dao_claim_transaction(
        &self,
        ctx: Context,
        payload: DaoClaimPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let from_item = Item::try_from(payload.clone().from)?;
        let to_address = match payload.to {
            Some(address) => match Address::from_str(&address) {
                Ok(address) => address,
                Err(error) => return Err(CoreError::InvalidRpcParams(error).into()),
            },
            None => self.get_secp_address_by_item(from_item.clone())?,
        };
        if !to_address.is_secp256k1() {
            return Err(CoreError::InvalidRpcParams(
                "Every to address should be secp/256k1 address".to_string(),
            )
            .into());
        }

        // get withdrawing cells including in lock period
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(AssetInfo::new_ckb());
        let cells = self
            .get_live_cells_by_item(
                ctx.clone(),
                from_item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                Some((**SECP256K1_CODE_HASH.load()).clone()),
                Some(ExtraType::Dao),
                false,
            )
            .await?;
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

        let mut inputs: Vec<packed::CellInput> = vec![];
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut script_set = HashSet::new();
        let mut signature_actions: HashMap<String, SignatureAction> = HashMap::new();
        let mut header_deps = vec![];
        let mut type_witness_args = HashMap::new();

        let mut header_dep_map = HashMap::new();
        let mut maximum_withdraw_capacity = 0;
        let mut last_input_index = 0;
        let from_address = self.get_secp_address_by_item(from_item)?;

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
                value: unlock_epoch,
            })?;

            // build input
            let input = packed::CellInputBuilder::default()
                .since(since.pack())
                .previous_output(withdrawing_cell.out_point.clone())
                .build();
            inputs.push(input);

            // build header deps
            let deposit_block_hash = deposit_cell.block_hash.pack();
            let withdrawing_block_hash = withdrawing_cell.block_hash.pack();
            if !header_dep_map.contains_key(&deposit_block_hash) {
                header_dep_map.insert(deposit_block_hash.clone(), header_deps.len());
                header_deps.push(deposit_block_hash.clone());
            }
            if !header_dep_map.contains_key(&withdrawing_block_hash) {
                header_dep_map.insert(withdrawing_block_hash.clone(), header_deps.len());
                header_deps.push(withdrawing_block_hash);
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
            type_witness_args.insert(
                inputs.len() - 1,
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

            // add signatures
            let lock_hash = withdrawing_cell.cell_output.calc_lock_hash().to_string();
            utils::add_signature_action(
                from_address.to_string(),
                lock_hash,
                SignAlgorithm::Secp256k1,
                HashAlgorithm::Blake2b,
                &mut signature_actions,
                last_input_index,
            );
            last_input_index += 1;
        }

        if inputs.is_empty() {
            return Err(CoreError::CannotFindUnlockedWithdrawingCell.into());
        }

        // build output cell
        let output_cell_capacity = maximum_withdraw_capacity - fixed_fee;
        let change_cell_index = self.build_cell_for_output(
            output_cell_capacity,
            to_address.payload().into(),
            None,
            None,
            &mut outputs,
            &mut cells_data,
        )?;

        // build resp
        script_set.insert(SECP256K1.to_string());
        script_set.insert(DAO.to_string());
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            header_deps,
            signature_actions,
            type_witness_args,
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_cell_index))
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
        utils::check_same_enum_value(payload.from.items.iter().collect())?;
        let mut payload = payload;
        payload.from.items = utils::dedup_json_items(payload.from.items);

        for to_info in &payload.to.to_infos {
            match u128::from_str(&to_info.amount) {
                Ok(amount) => {
                    if amount == 0u128 {
                        return Err(CoreError::TransferAmountMustPositive.into());
                    }
                }
                Err(_) => {
                    return Err(CoreError::InvalidRpcParams(
                        "To amount should be a valid u128 number".to_string(),
                    )
                    .into());
                }
            }
        }

        self.build_transaction_with_adjusted_fee(
            Self::prebuild_transfer_transaction,
            ctx,
            payload.clone(),
            payload.fee_rate,
        )
        .await
    }

    #[tracing_async]
    async fn prebuild_transfer_transaction(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        match (&payload.asset_info.asset_type, &payload.to.mode) {
            (AssetType::CKB, Mode::HoldByFrom) => {
                self.prebuild_secp_transfer_transaction(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::CKB, Mode::HoldByTo) => {
                self.prebuild_acp_transfer_transaction_with_ckb(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::HoldByFrom) => {
                self.prebuild_cheque_transfer_transaction(ctx.clone(), payload, fixed_fee)
                    .await
            }
            (AssetType::UDT, Mode::HoldByTo) => {
                self.prebuild_acp_transfer_transaction_with_udt(ctx.clone(), payload, fixed_fee)
                    .await
            }
        }
    }

    #[tracing_async]
    async fn prebuild_secp_transfer_transaction(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_actions: HashMap<String, SignatureAction> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(utils::address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    ctx.clone(),
                    items,
                    None,
                    required_ckb_part_1,
                    None,
                    None,
                    &mut inputs_part_1,
                    &mut script_set,
                    &mut signature_actions,
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
                .map_err(|err| CoreError::InvalidRpcParams(err.to_string()))?;
            if capacity < MIN_CKB_CAPACITY {
                return Err(CoreError::RequiredCKBLessThanMin.into());
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
                    ctx.clone(),
                    items,
                    Some(payload.from.source.clone()),
                    required_ckb_part_2 + fixed_fee,
                    payload.change,
                    None,
                    &mut inputs_part_2,
                    &mut script_set,
                    &mut signature_actions,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                ctx.clone(),
                items,
                Some(payload.from.source.clone()),
                required_ckb_part_2,
                payload.change,
                None,
                &mut inputs_part_2,
                &mut script_set,
                &mut signature_actions,
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
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
    }

    #[tracing_async]
    async fn prebuild_acp_transfer_transaction_with_ckb(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_actions: HashMap<String, SignatureAction> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(utils::address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    ctx.clone(),
                    items,
                    None,
                    required_ckb_part_1,
                    None,
                    None,
                    &mut inputs_part_1,
                    &mut script_set,
                    &mut signature_actions,
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
            let item = Item::Identity(utils::address_to_identity(&to.address)?);

            // build acp input
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
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }

            let current_capacity: u64 = live_acps[0].cell_output.capacity().unpack();
            inputs_part_2.push(live_acps[0].clone());
            input_index += 1;

            // build acp output
            let required_capacity = to
                .amount
                .parse::<u64>()
                .map_err(|err| CoreError::InvalidRpcParams(err.to_string()))?;
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
                    ctx.clone(),
                    from_items,
                    Some(payload.from.source.clone()),
                    required_ckb_part_2 + fixed_fee,
                    payload.change,
                    None,
                    &mut inputs_part_3,
                    &mut script_set,
                    &mut signature_actions,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                ctx.clone(),
                from_items,
                Some(payload.from.source.clone()),
                required_ckb_part_2,
                payload.change,
                None,
                &mut inputs_part_3,
                &mut script_set,
                &mut signature_actions,
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
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
    }

    #[tracing_async]
    async fn prebuild_cheque_transfer_transaction(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_actions: HashMap<String, SignatureAction> = HashMap::new();
        let mut change_fee_cell_index = 0usize;
        let mut input_index = 0;
        script_set.insert(SUDT.to_string());

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(utils::address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    ctx.clone(),
                    items,
                    None,
                    required_ckb_part_1,
                    None,
                    None,
                    &mut inputs_part_1,
                    &mut script_set,
                    &mut signature_actions,
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
                Address::from_str(&to.address).map_err(CoreError::InvalidRpcParams)?;
            if !receiver_address.is_secp256k1() {
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
            let to_udt_amount = to
                .amount
                .parse::<u128>()
                .map_err(|err| CoreError::InvalidRpcParams(err.to_string()))?;
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
            ctx.clone(),
            from_items.clone(),
            Some(payload.from.source.clone()),
            payload.asset_info.udt_hash.clone(),
            required_udt,
            &mut pool_udt_amount,
            &mut inputs_part_3,
            &mut script_set,
            &mut signature_actions,
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
                    ctx.clone(),
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
                    &mut signature_actions,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                ctx.clone(),
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
                &mut signature_actions,
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
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
    }

    #[tracing_async]
    async fn prebuild_acp_transfer_transaction_with_udt(
        &self,
        ctx: Context,
        payload: TransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let mut script_set = HashSet::new();
        let (mut outputs, mut cells_data) = (vec![], vec![]);
        let mut signature_actions: HashMap<String, SignatureAction> = HashMap::new();
        let mut change_fee_cell_index = 0;
        let mut input_index = 0;
        script_set.insert(SUDT.to_string());

        // tx part I: build pay fee input and change output
        let mut inputs_part_1 = vec![];
        let mut required_ckb_part_1 = 0;

        if let Some(ref pay_address) = payload.pay_fee {
            let items = vec![Item::Identity(utils::address_to_identity(pay_address)?)];
            required_ckb_part_1 += fixed_fee;
            change_fee_cell_index = self
                .build_required_ckb_and_change_tx_part(
                    ctx.clone(),
                    items,
                    None,
                    required_ckb_part_1,
                    None,
                    None,
                    &mut inputs_part_1,
                    &mut script_set,
                    &mut signature_actions,
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
            let item = Item::Identity(utils::address_to_identity(&to.address)?);

            // build acp input
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
            if live_acps.is_empty() {
                return Err(CoreError::CannotFindACPCell.into());
            }
            let existing_udt_amount = decode_udt_amount(&live_acps[0].cell_data);
            inputs_part_2.push(live_acps[0].clone());
            input_index += 1;
            script_set.insert(ACP.to_string());

            // build acp output
            let to_udt_amount = to
                .amount
                .parse::<u128>()
                .map_err(|err| CoreError::InvalidRpcParams(err.to_string()))?;
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
            ctx.clone(),
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
            &mut signature_actions,
            &mut input_index,
        )
        .await?;

        for cell in &inputs_part_3 {
            let udt_amount = decode_udt_amount(&cell.cell_data);
            pool_udt_amount += udt_amount;

            let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            if code_hash == **CHEQUE_CODE_HASH.load() {
                let address = match self
                    .generate_ckb_address_or_lock_hash(ctx.clone(), cell)
                    .await?
                {
                    AddressOrLockHash::Address(address) => address,
                    AddressOrLockHash::LockHash(_) => {
                        return Err(CoreError::CannotFindAddressByH160.into())
                    }
                };
                let address = Address::from_str(&address).map_err(CoreError::InvalidRpcParams)?;
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
                    ctx.clone(),
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
                    &mut signature_actions,
                    &mut outputs,
                    &mut cells_data,
                    &mut input_index,
                )
                .await?;
        } else {
            self.build_required_ckb_and_change_tx_part(
                ctx.clone(),
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
                &mut signature_actions,
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
        self.prebuild_tx_complete(
            inputs,
            outputs,
            cells_data,
            script_set,
            vec![],
            signature_actions,
            HashMap::new(),
        )
        .map(|(tx_view, signature_actions)| (tx_view, signature_actions, change_fee_cell_index))
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
            payload.fee_rate,
        )
        .await
    }

    async fn prebuild_simple_transfer_transaction(
        &self,
        ctx: Context,
        payload: SimpleTransferPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        if payload.from.is_empty() || payload.to.is_empty() {
            return Err(CoreError::NeedAtLeastOneFromAndOneTo.into());
        }
        if payload.from.len() > MAX_ITEM_NUM || payload.to.len() > MAX_ITEM_NUM {
            return Err(CoreError::ExceedMaxItemNum.into());
        }

        let mut from_items = vec![];
        for address in &payload.from {
            let identity = utils::address_to_identity(address)?;
            from_items.push(JsonItem::Identity(identity.encode()));
        }
        from_items = utils::dedup_json_items(from_items);

        let mut to_items = vec![];
        for ToInfo { address, .. } in &payload.to {
            let identity = utils::address_to_identity(address)?;
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
                self.prebuild_secp_transfer_transaction(ctx.clone(), transfer_payload, fixed_fee)
                    .await
            }

            AssetType::UDT => {
                let mut asset_infos = HashSet::new();
                asset_infos.insert(payload.asset_info.clone());
                let mode = self
                    .get_simple_transfer_mode(ctx.clone(), &to_items, asset_infos.clone())
                    .await?;
                let source = self
                    .get_simple_transfer_source(ctx.clone(), &from_items, &payload.to, asset_infos)
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
                        self.prebuild_cheque_transfer_transaction(
                            ctx.clone(),
                            transfer_payload,
                            fixed_fee,
                        )
                        .await
                    }
                    Mode::HoldByTo => {
                        if Source::Claimable == source {
                            transfer_payload.pay_fee = Some(payload.to[0].address.clone());
                        }
                        self.prebuild_acp_transfer_transaction_with_udt(
                            ctx.clone(),
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
    async fn build_transaction_with_adjusted_fee<'a, F, Fut, T>(
        &'a self,
        prebuild: F,
        ctx: Context,
        payload: T,
        fee_rate: Option<u64>,
    ) -> InnerResult<TransactionCompletionResponse>
    where
        F: Fn(&'a MercuryRpcImpl<C>, Context, T, u64) -> Fut + Copy,
        Fut: std::future::Future<
            Output = InnerResult<(TransactionView, Vec<SignatureAction>, usize)>,
        >,
        T: Clone,
    {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let (tx_view, signature_actions, change_cell_index) =
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
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, signature_actions);
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
            let live_acps = self
                .get_live_cells_by_item(
                    ctx.clone(),
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

    #[tracing_async]
    async fn get_simple_transfer_source(
        &self,
        ctx: Context,
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
            let resp = self.inner_get_balance(ctx.clone(), payload).await?;

            for b in resp.balances {
                claimable_amount += b
                    .claimable
                    .parse::<u128>()
                    .map_err(|e| CoreError::InvalidRpcParams(e.to_string()))?;
                free_amount += b
                    .free
                    .parse::<u128>()
                    .map_err(|e| CoreError::InvalidRpcParams(e.to_string()))?;
            }
        }

        for to in to_infos {
            required_amount += to
                .amount
                .parse::<u128>()
                .map_err(|e| CoreError::InvalidRpcParams(e.to_string()))?;
        }

        if claimable_amount >= required_amount {
            Ok(Source::Claimable)
        } else if free_amount >= required_amount {
            Ok(Source::Free)
        } else {
            Err(CoreError::UDTIsNotEnough.into())
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

    pub(crate) fn prebuild_tx_complete(
        &self,
        inputs: Vec<packed::CellInput>,
        outputs: Vec<packed::CellOutput>,
        cells_data: Vec<packed::Bytes>,
        script_set: HashSet<String>,
        header_deps: Vec<packed::Byte32>,
        signature_actions: HashMap<String, SignatureAction>,
        type_witness_args: HashMap<usize, (packed::BytesOpt, packed::BytesOpt)>,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>)> {
        // build cell deps
        let cell_deps = self.build_cell_deps(script_set)?;

        // build witnesses
        let mut witnesses_map = HashMap::new();
        for sig_action in signature_actions.values() {
            match sig_action.signature_info.algorithm {
                SignAlgorithm::Secp256k1 => {
                    let mut witness = packed::WitnessArgs::new_builder()
                        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                        .build();
                    if let Some((input_type, output_type)) =
                        type_witness_args.get(&sig_action.signature_location.index)
                    {
                        witness = witness
                            .as_builder()
                            .input_type(input_type.to_owned())
                            .output_type(output_type.to_owned())
                            .build()
                    };
                    witnesses_map.insert(sig_action.signature_location.index, witness);

                    for other_index in &sig_action.other_indexes_in_group {
                        let mut witness = packed::WitnessArgs::new_builder().build();
                        if let Some((input_type, output_type)) = type_witness_args.get(other_index)
                        {
                            witness = witness
                                .as_builder()
                                .input_type(input_type.to_owned())
                                .output_type(output_type.to_owned())
                                .build()
                        }
                        witnesses_map.insert(*other_index, witness);
                    }
                }
            };
        }
        let mut witnesses = vec![];
        for (index, _) in inputs.iter().enumerate() {
            if let Some(witness) = witnesses_map.get(&index) {
                witnesses.push(witness.as_bytes().pack());
            } else {
                witnesses.push(packed::WitnessArgs::new_builder().build().as_bytes().pack());
            }
        }

        // build tx view
        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .outputs(outputs)
            .outputs_data(cells_data)
            .inputs(inputs)
            .cell_deps(cell_deps)
            .header_deps(header_deps)
            .witnesses(witnesses)
            .build();

        let mut signature_actions: Vec<SignatureAction> =
            signature_actions.into_iter().map(|(_, s)| s).collect();
        signature_actions.sort_unstable();

        Ok((tx_view, signature_actions))
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

    fn build_cell_deps(&self, script_set: HashSet<String>) -> InnerResult<Vec<packed::CellDep>> {
        let mut deps = Vec::new();
        for s in script_set.iter() {
            deps.push(
                self.builtin_scripts
                    .get(s)
                    .cloned()
                    .ok_or_else(|| CoreError::MissingScriptInfo(s.clone()))?
                    .cell_dep,
            )
        }
        Ok(deps)
    }

    pub(crate) fn build_tx_cell_inputs(
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

    #[tracing_async]
    async fn build_required_ckb_and_change_tx_part(
        &self,
        ctx: Context,
        items: Vec<Item>,
        source: Option<Source>,
        required_ckb: u64,
        change_address: Option<String>,
        udt_change_info: Option<UDTInfo>,
        inputs: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_actions: &mut HashMap<String, SignatureAction>,
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
            ctx.clone(),
            items.to_owned(),
            required_ckb,
            vec![],
            source,
            &mut 0,
            inputs,
            script_set,
            signature_actions,
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
                    .build_sudt_type_script(
                        ctx.clone(),
                        blake2b_256_to_160(&udt_info.asset_info.udt_hash),
                    )
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

    #[tracing_async]
    async fn build_required_udt_tx_part(
        &self,
        ctx: Context,
        from_items: Vec<Item>,
        source: Option<Source>,
        udt_hash: H256,
        required_udt: u128,
        pool_udt_amount: &mut u128,
        inputs: &mut Vec<DetailedCell>,
        script_set: &mut HashSet<String>,
        signature_actions: &mut HashMap<String, SignatureAction>,
        outputs: &mut Vec<packed::CellOutput>,
        cells_data: &mut Vec<packed::Bytes>,
        input_index: &mut usize,
    ) -> InnerResult<()> {
        self.pool_live_cells_by_items(
            ctx.clone(),
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
            signature_actions,
            input_index,
        )
        .await?;

        for cell in inputs {
            let udt_amount = decode_udt_amount(&cell.cell_data);
            *pool_udt_amount += udt_amount;

            let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
            if code_hash == **CHEQUE_CODE_HASH.load() {
                let address = match self
                    .generate_ckb_address_or_lock_hash(ctx.clone(), cell)
                    .await?
                {
                    AddressOrLockHash::Address(address) => address,
                    AddressOrLockHash::LockHash(_) => {
                        return Err(CoreError::CannotFindAddressByH160.into())
                    }
                };
                let address = Address::from_str(&address).map_err(CoreError::InvalidRpcParams)?;
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

pub(crate) fn calculate_tx_size(tx_view: TransactionView) -> usize {
    let tx_size = tx_view.data().total_size();
    // tx offset bytesize
    tx_size + 4
}
