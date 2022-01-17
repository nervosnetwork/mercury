use crate::r#impl::utils_types::TransferComponents;
use crate::r#impl::MercuryRpcImpl;
use crate::{error::CoreError, InnerResult};

use ckb_types::prelude::*;
use ckb_types::{bytes::Bytes, core::TransactionView, packed};
use ckb_types::{H160, H256};

use ckb_types::core::Capacity;
use common::utils::{decode_udt_amount, parse_address, to_fixed_array};
use common::{Address, AddressPayload, Context, NetworkType, ACP, SUDT};
use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{
    generated, pack_u128, unpack_byte16, CrossChainTransferPayload, InitChainPayload,
    IssueAssetPayload, AXON_SELECTION_LOCK,
};
use core_rpc_types::consts::{BYTE_SHANNONS, OMNI_SCRIPT};
use core_rpc_types::{
    HashAlgorithm, Item, SignAlgorithm, SignatureAction, SignatureInfo, SignatureLocation, Source,
    TransactionCompletionResponse,
};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_cross_chain_transfer_tx(
        &self,
        ctx: Context,
        payload: CrossChainTransferPayload,
    ) -> InnerResult<TransactionCompletionResponse> {
        let mut inputs = Vec::new();
        let sender = parse_address(&payload.sender)
            .map_err(|e| CoreError::ParseAddressError(e.to_string()))?;
        let receiver = parse_address(&payload.receiver)
            .map_err(|e| CoreError::ParseAddressError(e.to_string()))?;
        let amount: u128 = payload.amount.parse().unwrap();

        let input_user_cell = self
            .get_live_cells(
                ctx.clone(),
                None,
                vec![self
                    .build_acp_cell(sender.payload().args())
                    .calc_script_hash()
                    .unpack()],
                vec![payload.udt_hash.clone()],
                None,
                None,
                Default::default(),
            )
            .await?
            .response
            .first()
            .cloned()
            .unwrap();
        let input_relayer_cell = self
            .get_live_cells(
                ctx.clone(),
                None,
                vec![self
                    .build_acp_cell(receiver.payload().args())
                    .calc_script_hash()
                    .unpack()],
                vec![payload.udt_hash],
                None,
                None,
                Default::default(),
            )
            .await?
            .response
            .first()
            .cloned()
            .unwrap();

        let user_capacity: u64 = input_user_cell.cell_output.capacity().unpack();
        let output_user_cell = input_user_cell
            .cell_output
            .clone()
            .as_builder()
            .capacity((user_capacity - 1000).pack())
            .build();
        let output_user_cell_data = (decode_udt_amount(&input_user_cell.cell_data)
            .checked_sub(amount)
            .unwrap())
        .to_le_bytes()
        .to_vec();
        let output_relayer_cell = input_relayer_cell.cell_output.clone();
        let output_releyer_cell_data = (decode_udt_amount(&input_relayer_cell.cell_data)
            .checked_add(amount)
            .unwrap())
        .to_le_bytes()
        .to_vec();

        let sig_action = SignatureAction {
            signature_location: SignatureLocation {
                index: 0,
                offset: SignAlgorithm::Secp256k1.get_signature_offset().0,
            },
            signature_info: SignatureInfo {
                algorithm: SignAlgorithm::Secp256k1,
                address: payload.sender,
            },
            hash_algorithm: HashAlgorithm::Blake2b,
            other_indexes_in_group: vec![],
        };

        let mut transfer_component = TransferComponents::new();
        inputs.push(
            packed::CellInputBuilder::default()
                .previous_output(input_user_cell.out_point.clone())
                .build(),
        );
        inputs.push(
            packed::CellInputBuilder::default()
                .previous_output(input_relayer_cell.out_point)
                .build(),
        );
        transfer_component.outputs.push(output_user_cell);
        transfer_component
            .outputs_data
            .push(output_user_cell_data.pack());
        transfer_component.outputs.push(output_relayer_cell);
        transfer_component
            .outputs_data
            .push(output_releyer_cell_data.pack());
        transfer_component.script_deps.insert(ACP.to_string());
        transfer_component.script_deps.insert(SUDT.to_string());
        transfer_component.signature_actions.insert(
            input_user_cell.cell_output.calc_lock_hash().to_string(),
            sig_action,
        );

        let (tx_view, signature_actions) = self.prebuild_tx_complete(
            inputs,
            transfer_component.outputs,
            transfer_component.outputs_data,
            transfer_component.script_deps,
            transfer_component.header_deps,
            transfer_component.signature_actions,
            transfer_component.type_witness_args,
        )?;

        let mut witnesses = unpack_output_data_vec(tx_view.witnesses());
        witnesses.push(payload.memo.as_bytes().pack());

        Ok(TransactionCompletionResponse::new(
            tx_view
                .as_advanced_builder()
                .set_witnesses(witnesses)
                .build()
                .into(),
            signature_actions,
        ))
    }

    pub(crate) async fn prebuild_issue_asset_tx(
        &self,
        ctx: Context,
        payload: IssueAssetPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let input_omni_cell = self
            .get_live_cells(
                ctx.clone(),
                None,
                vec![],
                vec![payload.omni_type_hash.clone()],
                None,
                None,
                Default::default(),
            )
            .await?
            .response
            .first()
            .cloned()
            .ok_or_else(|| CoreError::CannotFindCell(OMNI_SCRIPT.to_string()))?;
        println!("input omni cell {:?}", input_omni_cell);
        let input_selection_cell = self
            .get_live_cells(
                ctx.clone(),
                None,
                vec![payload.selection_lock_hash.clone()],
                vec![],
                None,
                None,
                Default::default(),
            )
            .await?
            .response
            .first()
            .cloned()
            .ok_or_else(|| CoreError::CannotFindCell(AXON_SELECTION_LOCK.to_string()))?;
        println!("input selection cell {:?}", input_selection_cell);

        let mint_amount: u128 = payload.amount.parse().unwrap();
        let mut omni_data = input_omni_cell.cell_data.clone().to_vec();
        let new_supply = u128::from_le_bytes(to_fixed_array(&omni_data[1..17])) + mint_amount;
        omni_data[1..17].swap_with_slice(&mut new_supply.to_le_bytes());

        let acp_data = Bytes::from(mint_amount.to_le_bytes().to_vec());
        let acp_cell =
            packed::CellOutputBuilder::default()
                .type_(
                    Some(self.build_sudt_script(
                        input_selection_cell.cell_output.lock().calc_script_hash(),
                    ))
                    .pack(),
                )
                .lock(
                    self.build_acp_cell(
                        hex::decode(&payload.admin_id.content.to_vec()[2..])
                            .unwrap()
                            .into(),
                    ),
                )
                .build_exact_capacity(Capacity::shannons(acp_data.len() as u64 * BYTE_SHANNONS))
                .unwrap();

        let admin_address = H160::from_slice(
            &input_omni_cell
                .cell_output
                .lock()
                .args()
                .raw_data()
                .to_vec()[1..21],
        )
        .unwrap();
        let sig_action = SignatureAction {
            signature_location: SignatureLocation {
                index: 1,
                offset: SignAlgorithm::Secp256k1.get_signature_offset().0 + 20,
            },
            signature_info: SignatureInfo {
                algorithm: SignAlgorithm::Secp256k1,
                address: Address::new(
                    NetworkType::Testnet,
                    AddressPayload::from_pubkey_hash(admin_address),
                    true,
                )
                .to_string(),
            },
            hash_algorithm: HashAlgorithm::Blake2b,
            other_indexes_in_group: vec![],
        };

        let mut transfer_component = TransferComponents::new();
        transfer_component.inputs.push(input_selection_cell.clone());
        transfer_component.inputs.push(input_omni_cell.clone());
        transfer_component
            .outputs
            .push(input_selection_cell.cell_output);
        transfer_component.outputs_data.push(Default::default());
        transfer_component
            .outputs
            .push(input_omni_cell.cell_output.clone());
        transfer_component.outputs_data.push(omni_data.pack());
        transfer_component.outputs.push(acp_cell);
        transfer_component.outputs_data.push(acp_data.pack());
        transfer_component.signature_actions.insert(
            input_omni_cell.cell_output.calc_lock_hash().to_string(),
            sig_action,
        );
        transfer_component
            .script_deps
            .insert(AXON_SELECTION_LOCK.to_string());
        transfer_component
            .script_deps
            .insert(OMNI_SCRIPT.to_string());

        self.balance_transfer_tx_capacity(
            ctx.clone(),
            vec![Item::Identity(payload.admin_id.try_into().unwrap())],
            &mut transfer_component,
            Some(fixed_fee),
            None,
        )
        .await?;
        let inputs = self.build_transfer_tx_cell_inputs(
            &transfer_component.inputs,
            None,
            transfer_component.dao_since_map,
            Source::Free,
        )?;

        let fee_change_cell_index = transfer_component
            .fee_change_cell_index
            .ok_or(CoreError::InvalidFeeChange)?;
        let (tx_view, signature_actions) = self.prebuild_tx_complete(
            inputs,
            transfer_component.outputs,
            transfer_component.outputs_data,
            transfer_component.script_deps,
            transfer_component.header_deps,
            transfer_component.signature_actions,
            transfer_component.type_witness_args,
        )?;

        let mut witnesses = unpack_output_data_vec(tx_view.witnesses());
        let omni_witness = generated::RcLockWitnessLockBuilder::default()
            .signature(
                generated::BytesOptBuilder::default()
                    .set(build_bytes_opt([0u8; 65].to_vec().into()))
                    .build(),
            )
            .build()
            .as_bytes();
        witnesses[1] = packed::WitnessArgsBuilder::default()
            .lock(Some(omni_witness).pack())
            .build()
            .as_bytes()
            .pack();

        Ok((
            tx_view
                .as_advanced_builder()
                .set_witnesses(witnesses)
                .build(),
            signature_actions,
            fee_change_cell_index,
        ))
    }

    pub(crate) async fn prebuild_init_axon_chain_tx(
        &self,
        ctx: Context,
        payload: InitChainPayload,
        fixed_fee: u64,
    ) -> InnerResult<(TransactionView, Vec<SignatureAction>, usize)> {
        let (omni_cell, omni_cell_data) =
            self.build_omni_cell(payload.omni_config.clone(), payload.admin_id.clone())?;
        let (checkpoint_cell, checkpoint_cell_data) = self
            .build_checkpoint_cell(payload.check_point_config.clone(), payload.admin_id.clone())?;
        let (stake_cell, stake_cell_data) =
            self.build_stake_cell(payload.state_config.clone(), payload.admin_id.clone())?;
        let selection_cell =
            self.build_selection_cell(checkpoint_cell.lock().calc_script_hash().unpack())?;

        let mut transfer_component = TransferComponents::new();
        transfer_component.outputs.push(selection_cell);
        transfer_component.outputs_data.push(Default::default());
        transfer_component.outputs.push(omni_cell);
        transfer_component.outputs_data.push(omni_cell_data.pack());
        transfer_component.outputs.push(checkpoint_cell);
        transfer_component
            .outputs_data
            .push(checkpoint_cell_data.pack());
        transfer_component.outputs.push(stake_cell);
        transfer_component.outputs_data.push(stake_cell_data.pack());

        self.balance_transfer_tx_capacity(
            ctx.clone(),
            vec![Item::Identity(payload.admin_id.try_into().unwrap())],
            &mut transfer_component,
            Some(fixed_fee),
            None,
        )
        .await?;

        let inputs = self.build_transfer_tx_cell_inputs(
            &transfer_component.inputs,
            None,
            transfer_component.dao_since_map,
            Source::Free,
        )?;
        let first_input_cell = inputs.get(0).cloned().unwrap();
        let fee_change_cell_index = transfer_component
            .fee_change_cell_index
            .ok_or(CoreError::InvalidFeeChange)?;
        let (tx_view, signature_actions) = self.prebuild_tx_complete(
            inputs,
            transfer_component.outputs,
            transfer_component.outputs_data,
            transfer_component.script_deps,
            transfer_component.header_deps,
            transfer_component.signature_actions,
            transfer_component.type_witness_args,
        )?;

        let mut output_cell_vec = unpack_output_vec(tx_view.outputs());
        let mut output_cell_data_vec = unpack_output_data_vec(tx_view.outputs_data());

        // Update omni cell
        let omni_type_script = self.build_type_id_script(&first_input_cell, 1)?;
        let omni_type_hash = omni_type_script.calc_script_hash();
        output_cell_vec[1] = output_cell_vec[1]
            .clone()
            .as_builder()
            .type_(Some(omni_type_script).pack())
            .build();
        let mut omni_lock_args = tx_view.output(1).unwrap().lock().args().raw_data().to_vec();
        omni_lock_args[22..].swap_with_slice(&mut omni_type_hash.raw_data().to_vec());
        let omni_lock = output_cell_vec[1]
            .lock()
            .as_builder()
            .args(omni_lock_args.pack())
            .build();
        output_cell_vec[1] = output_cell_vec[1]
            .clone()
            .as_builder()
            .lock(omni_lock)
            .build();

        // Update checkpoint cell
        let checkpoint_type_script = self.build_type_id_script(&first_input_cell, 2)?;
        let checkpoint_type_hash = checkpoint_type_script.calc_script_hash();
        output_cell_vec[2] = output_cell_vec[2]
            .clone()
            .as_builder()
            .type_(Some(checkpoint_type_script).pack())
            .build();
        let checkpoint_lock_args = tx_view.output(2).unwrap().lock().args().raw_data();
        let new_args = generated::CheckpointLockArgs::new_unchecked(checkpoint_lock_args)
            .as_builder()
            .type_id_hash(checkpoint_type_hash.into())
            .build();
        let checkpoint_lock = output_cell_vec[2]
            .lock()
            .as_builder()
            .args(new_args.as_bytes().pack())
            .build();
        output_cell_vec[2] = output_cell_vec[2]
            .clone()
            .as_builder()
            .lock(checkpoint_lock)
            .build();

        // Update stake cell
        let stake_type_script = self.build_type_id_script(&first_input_cell, 3)?;
        let stake_type_hash = stake_type_script.calc_script_hash();
        output_cell_vec[3] = output_cell_vec[3]
            .clone()
            .as_builder()
            .type_(Some(stake_type_script).pack())
            .build();
        let stake_lock_args = tx_view.output(3).unwrap().lock().args().raw_data();
        let new_args = generated::StakeLockArgs::new_unchecked(stake_lock_args)
            .as_builder()
            .type_id_hash(stake_type_hash.clone().into())
            .build();
        let stake_lock_script = output_cell_vec[3]
            .lock()
            .as_builder()
            .args(new_args.as_bytes().pack())
            .build();
        output_cell_vec[3] = output_cell_vec[3]
            .clone()
            .as_builder()
            .lock(stake_lock_script)
            .build();

        // Update selection cell
        let omni_lock_hash = output_cell_vec[1].lock().calc_script_hash();
        let checkpoint_lock_hash = output_cell_vec[2].lock().calc_script_hash();
        let new_args = generated::SelectionLockArgsBuilder::default()
            .omni_lock_hash(omni_lock_hash.into())
            .checkpoint_lock_hash(checkpoint_lock_hash.into())
            .build();
        let selection_lock_script = output_cell_vec[0]
            .lock()
            .as_builder()
            .args(new_args.as_bytes().pack())
            .build();
        output_cell_vec[0] = output_cell_vec[0]
            .clone()
            .as_builder()
            .lock(selection_lock_script)
            .build();

        let sudt_args = output_cell_vec[0].lock().calc_script_hash();
        let sudt_type_hash = self.build_sudt_script(sudt_args).calc_script_hash();

        // Updata omni data
        let mut omni_data = tx_view.outputs_data().get_unchecked(1).raw_data().to_vec();
        omni_data[33..].swap_with_slice(&mut sudt_type_hash.raw_data().to_vec());
        output_cell_data_vec[1] = omni_data.pack();

        // Update checkpoint data
        let checkpoint_data = tx_view.outputs_data().get_unchecked(2).raw_data();
        let new_data = generated::CheckpointLockCellData::new_unchecked(checkpoint_data)
            .as_builder()
            .sudt_type_hash(sudt_type_hash.clone().into())
            .stake_type_hash(stake_type_hash.clone().into())
            .build()
            .as_bytes();
        output_cell_data_vec[2] = new_data.pack();

        // Updata stake data
        let stake_data = tx_view.outputs_data().get_unchecked(3).raw_data();
        let new_data = generated::StakeLockCellData::new_unchecked(stake_data)
            .as_builder()
            .sudt_type_hash(sudt_type_hash.into())
            .build()
            .as_bytes();
        output_cell_data_vec[3] = new_data.pack();

        Ok((
            tx_view
                .as_advanced_builder()
                .set_outputs(output_cell_vec)
                .set_outputs_data(output_cell_data_vec)
                .build(),
            signature_actions,
            fee_change_cell_index,
        ))
    }
}

fn convert_bytes(input: Bytes) -> Vec<packed::Byte> {
    input.into_iter().map(|i| packed::Byte::new(i)).collect()
}

fn build_bytes_opt(input: Bytes) -> Option<generated::Bytes> {
    let bytes = convert_bytes(input);
    let bytes = generated::BytesBuilder::default().extend(bytes).build();
    Some(bytes)
}

fn unpack_output_vec(outputs: packed::CellOutputVec) -> Vec<packed::CellOutput> {
    outputs.into_iter().collect()
}

fn unpack_output_data_vec(outputs_data: packed::BytesVec) -> Vec<packed::Bytes> {
    outputs_data.into_iter().collect()
}
