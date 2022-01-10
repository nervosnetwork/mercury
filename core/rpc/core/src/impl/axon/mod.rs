use crate::r#impl::MercuryRpcImpl;
use crate::{error::CoreError, InnerResult};

use ckb_types::prelude::*;
use ckb_types::{bytes::Bytes, packed, H256};

use common::hash::blake2b_256;
use common::Context;
use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{
    generated, pack_u128, pack_u32, pack_u64, to_packed_array, CheckpointConfig, Identity,
    InitChainPayload, OmniConfig, SidechainConfig, AXON_CHECKPOINT_LOCK, AXON_SELECTION_LOCK,
};
use core_rpc_types::consts::{OMNI_SCRIPT, TYPE_ID_SCRIPT};
use core_rpc_types::TransactionCompletionResponse;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    fn inner_build_init_axon_chain_tx(
        &self,
        ctx: Context,
        payload: InitChainPayload,
    ) -> InnerResult<(TransactionCompletionResponse, SidechainConfig)> {
        todo!()
    }

    fn build_omni_cell(
        &self,
        omni_config: OmniConfig,
        input_0_outpoint: packed::OutPoint,
        admin_identity: Identity,
    ) -> InnerResult<(packed::CellOutput, Bytes)> {
        let mut type_args = input_0_outpoint.as_bytes().to_vec();
        type_args.push(1);
        let type_args = blake2b_256(type_args).to_vec();

        let type_script = self
            .builtin_scripts
            .get(TYPE_ID_SCRIPT)
            .ok_or(CoreError::MissingAxonCellInfo(TYPE_ID_SCRIPT.to_string()))?
            .script
            .clone()
            .as_builder()
            .args(type_args.pack())
            .build();

        let lock_args = generated::OmniLockArgsBuilder::default()
            .identity(
                generated::Identity::try_from(admin_identity)
                    .map_err(|e| CoreError::DecodeHexError(e))?,
            )
            .flag(packed::Byte::new(8))
            .build();
        let lock_script = self
            .builtin_scripts
            .get(OMNI_SCRIPT)
            .ok_or(CoreError::MissingAxonCellInfo(TYPE_ID_SCRIPT.to_string()))?
            .script
            .clone()
            .as_builder()
            .args(lock_args.as_bytes().pack())
            .build();

        let data = generated::OmniDataBuilder::default()
            .version(packed::Byte::new(0))
            .current_supply(pack_u128(0))
            .max_supply(pack_u128(omni_config.max_supply.try_into().unwrap()))
            .build();

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(Some(type_script).pack())
                .build(),
            data.as_bytes(),
        ))
    }

    fn build_selection_cell(
        &self,
        omni_lock_hash: H256,
        checkpoint_lock_hash: H256,
    ) -> InnerResult<packed::CellOutput> {
        let lock_script = self
            .builtin_scripts
            .get(AXON_SELECTION_LOCK)
            .ok_or(CoreError::MissingAxonCellInfo(TYPE_ID_SCRIPT.to_string()))?
            .script
            .clone()
            .as_builder()
            .args(
                generated::SelectionLockArgsBuilder::default()
                    .omni_lock_hash(omni_lock_hash.pack().into())
                    .checkpoint_lock_hash(checkpoint_lock_hash.pack().into())
                    .build()
                    .as_bytes()
                    .pack(),
            )
            .build();

        todo!()
    }

    fn build_checkpoint_cell(
        &self,
        admin_identity: Identity,
        checkpoint_config: CheckpointConfig,
    ) -> InnerResult<(packed::CellOutput, Bytes)> {
        let type_script = self
            .builtin_scripts
            .get(TYPE_ID_SCRIPT)
            .ok_or(CoreError::MissingAxonCellInfo(TYPE_ID_SCRIPT.to_string()))?
            .script
            .clone()
            .as_builder()
            // .args()
            .build();

        let lock_script = self
            .builtin_scripts
            .get(AXON_CHECKPOINT_LOCK)
            .ok_or(CoreError::MissingAxonCellInfo(
                AXON_CHECKPOINT_LOCK.to_string(),
            ))?
            .script
            .clone()
            .as_builder()
            .args(
                generated::Identity::try_from(admin_identity)
                    .map_err(|e| CoreError::DecodeHexError(e))?
                    .as_bytes()
                    .pack(),
            )
            .build();

        let data = generated::CheckpointLockCellDataBuilder::default()
            .version(packed::Byte::new(checkpoint_config.version))
            .state(packed::Byte::new(0))
            .period(pack_u64(0))
            .era(pack_u64(0))
            .block_hash(Default::default())
            .period_interval(pack_u32(checkpoint_config.period_intervial))
            .era_period(pack_u32(checkpoint_config.era_period))
            .unlock_period(pack_u32(0))
            .base_reward(pack_u128(checkpoint_config.base_reward.try_into().unwrap()))
            .half_period(pack_u64(checkpoint_config.half_period))
            .common_ref(
                generated::Byte10Builder::default()
                    .set(to_packed_array::<10>(&checkpoint_config.common_ref))
                    .build(),
            )
            .withdrawal_lock_code_hash(checkpoint_config.withdrawal_lock_hash.pack().into())
            .build();

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(Some(type_script).pack())
                .build(),
            data.as_bytes(),
        ))
    }
}
