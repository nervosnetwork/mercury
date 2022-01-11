use crate::r#impl::utils_types::{CkbCellsCache, TransferComponents};
use crate::r#impl::MercuryRpcImpl;
use crate::{error::CoreError, InnerResult};

use ckb_types::prelude::*;
use ckb_types::{bytes::Bytes, core::TransactionView, packed, H256};

use common::hash::blake2b_256;
use common::Context;
use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{
    generated, pack_u128, pack_u64, to_packed_array, unpack_byte16, CheckpointConfig, Identity,
    InitChainPayload, IssueAssetPayload, OmniConfig, SidechainConfig, StakeConfig,
    AXON_CHECKPOINT_LOCK, AXON_SELECTION_LOCK,
};
use core_rpc_types::consts::{BYTE_SHANNONS, TYPE_ID_SCRIPT};
use core_rpc_types::{
    AssetInfo, AssetType, Item, SignatureAction, Source, TransactionCompletionResponse,
};

impl<C: CkbRpc> MercuryRpcImpl<C> {
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

        let mint_amount: u128 = payload.amount.try_into().unwrap();
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

        todo!()
    }

    async fn prebuild_init_axon_chain_tx(
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
        let selection_cell = self.build_selection_cell(
            omni_cell.lock().calc_script_hash().unpack(),
            checkpoint_cell.lock().calc_script_hash().unpack(),
        )?;

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
        let fee_change_cell_index = transfer_component
            .fee_change_cell_index
            .ok_or(CoreError::InvalidFeeChange)?;
        let (mut tx_view, signature_actions) = self.prebuild_tx_complete(
            inputs, 
            transfer_component.outputs,
            transfer_component.outputs_data,
            transfer_component.script_deps,
            transfer_component.header_deps,
            transfer_component.signature_actions,
            transfer_component.type_witness_args,
        )?;

        

        Ok((tx_view, signature_actions, fee_change_cell_index))
    }
}
