use crate::r#impl::utils_types::TransferComponents;
use crate::r#impl::MercuryRpcImpl;
use crate::{error::CoreError, InnerResult};

use ckb_types::prelude::*;
use ckb_types::{bytes::Bytes, core::TransactionView, packed};

use common::utils::{decode_udt_amount, parse_address};
use common::Context;
use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{
    generated, pack_u128, unpack_byte16, BuildCrossChainTransferTxPayload, InitChainPayload,
    IssueAssetPayload,
};
use core_rpc_types::{
    HashAlgorithm, Item, SignAlgorithm, SignatureAction, SignatureInfo, SignatureLocation, Source,
};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    async fn build_cross_chain_transfer_tx(
        &self,
        ctx: Context,
        payload: BuildCrossChainTransferTxPayload,
    ) -> InnerResult<(TransactionView, SignatureAction)> {
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

        let output_user_cell = input_user_cell.cell_output;
        let output_user_cell_data = (decode_udt_amount(&input_user_cell.cell_data)
            .checked_sub(amount)
            .unwrap())
        .to_le_bytes()
        .to_vec();
        let output_relayer_cell = input_relayer_cell.cell_output;
        let output_releyer_cell_data = (decode_udt_amount(&input_relayer_cell.cell_data)
            .checked_add(amount)
            .unwrap())
        .to_le_bytes()
        .to_vec();

        let sig_action = SignatureAction {
            signature_location: SignatureLocation {
                index: 0,
                offset: 1,
            },
            signature_info: SignatureInfo {
                algorithm: SignAlgorithm::Secp256k1,
                address: payload.sender,
            },
            hash_algorithm: HashAlgorithm::Blake2b,
            other_indexes_in_group: vec![],
        };

        todo!()
    }

    async fn prebuild_issue_asset_tx(
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
            .unwrap();
        let input_selection_cell = self
            .get_live_cells(
                ctx,
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
            .unwrap();

        let mint_amount: u128 = payload.amount.parse().unwrap();
        let omni_data = generated::OmniData::new_unchecked(input_selection_cell.cell_data);
        let new_supply = unpack_byte16(omni_data.current_supply()) + mint_amount;
        let omni_data = omni_data
            .as_builder()
            .current_supply(pack_u128(new_supply))
            .build()
            .as_bytes();

        let acp_cell = self
            .builtin_scripts
            .get("ACP")
            .unwrap()
            .script
            .clone()
            .as_builder();
        let acp_data = Bytes::from(mint_amount.to_le_bytes().to_vec());

        todo!()
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

        let first_input_cell = transfer_component.inputs.first().cloned().unwrap();

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

        let mut output_cell_vec = unpack_output_vec(tx_view.outputs());
        let mut output_cell_data_vec = unpack_output_data_vec(tx_view.outputs_data());

        // Update omni cell
        let omni_type_script = self.build_type_id_script(&first_input_cell.out_point, 1)?;
        let omni_type_hash = omni_type_script.calc_script_hash();
        output_cell_vec[1] = output_cell_vec[1]
            .clone()
            .as_builder()
            .type_(Some(omni_type_script).pack())
            .build();

        let omni_lock_args = tx_view.output(1).unwrap().lock().args().raw_data();
        let new_args = generated::OmniLockArgs::new_unchecked(omni_lock_args)
            .as_builder()
            .omni_type_hash(omni_type_hash.into())
            .build();
        let omni_lock = output_cell_vec[1]
            .lock()
            .as_builder()
            .args(new_args.as_bytes().pack())
            .build();
        output_cell_vec[1] = output_cell_vec[1]
            .clone()
            .as_builder()
            .lock(omni_lock)
            .build();

        // Update checkpoint cell
        let checkpoint_type_script = self.build_type_id_script(&first_input_cell.out_point, 2)?;
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
        let stake_type_script = self.build_type_id_script(&first_input_cell.out_point, 3)?;
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
        let omni_lock_hash = tx_view.output(1).unwrap().lock().calc_script_hash();
        let checkpoint_lock_hash = tx_view.output(2).unwrap().lock().calc_script_hash();
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
        let omni_data = tx_view.outputs_data().get_unchecked(1).raw_data();
        let new_data = generated::OmniData::new_unchecked(omni_data)
            .as_builder()
            .sudt_type_hash(sudt_type_hash.clone().into())
            .build()
            .as_bytes();
        output_cell_data_vec[1] = new_data.pack();

        // Updata checkpoint data
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

fn unpack_output_vec(outputs: packed::CellOutputVec) -> Vec<packed::CellOutput> {
    outputs.into_iter().collect()
}

fn unpack_output_data_vec(outputs_data: packed::BytesVec) -> Vec<packed::Bytes> {
    outputs_data.into_iter().collect()
}
