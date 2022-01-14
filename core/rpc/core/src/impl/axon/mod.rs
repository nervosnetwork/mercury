mod build_init;

use crate::r#impl::MercuryRpcImpl;
use crate::{error::CoreError, InnerResult};

use ckb_types::prelude::*;
use ckb_types::{bytes::Bytes, core::Capacity, packed, H256};

use common::hash::blake2b_256;
use common::{Context, ACP, SUDT, TYPE_ID_CODE_HASH};
use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{
    generated, pack_u128, pack_u32, pack_u64, to_packed_array, CheckpointConfig, Identity,
    InitChainPayload, InitChainResponse, OmniConfig, SidechainConfig, StakeConfig,
    AXON_CHECKPOINT_LOCK, AXON_SELECTION_LOCK,
};
use core_rpc_types::consts::{BYTE_SHANNONS, OMNI_SCRIPT, TYPE_ID_SCRIPT};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_init_side_chain(
        &self,
        ctx: Context,
        payload: InitChainPayload,
    ) -> InnerResult<InitChainResponse> {
        let tx = self
            .build_transaction_with_adjusted_fee(
                Self::prebuild_init_axon_chain_tx,
                ctx,
                payload,
                None,
            )
            .await?;

        let tx_view: packed::Transaction = tx.tx_view.inner.clone().into();
        let tx_view = tx_view.into_view();

        let config = SidechainConfig {
            omni_type_hash: tx_view
                .output(1)
                .unwrap()
                .type_()
                .to_opt()
                .unwrap()
                .calc_script_hash()
                .unpack(),
            checkpoint_type_hash: tx_view
                .output(2)
                .unwrap()
                .type_()
                .to_opt()
                .unwrap()
                .calc_script_hash()
                .unpack(),
            stake_type_hash: tx_view
                .output(3)
                .unwrap()
                .type_()
                .to_opt()
                .unwrap()
                .calc_script_hash()
                .unpack(),
            selection_lock_hash: tx_view
                .output(0)
                .unwrap()
                .lock()
                .calc_script_hash()
                .unpack(),
            udt_hash: H256::from_slice(
                &generated::StakeLockCellData::new_unchecked(
                    tx_view.outputs_data().get_unchecked(3).as_bytes(),
                )
                .sudt_type_hash()
                .raw_data(),
            )
            .unwrap(),
        };

        Ok(InitChainResponse { tx, config })
    }

    pub(crate) fn build_stake_cell(
        &self,
        stake_config: StakeConfig,
        admin_identity: Identity,
    ) -> InnerResult<(packed::CellOutput, Bytes)> {
        let type_script = packed::ScriptBuilder::default()
            .args(H256::default().0.to_vec().pack())
            .build();

        let lock_args = generated::StakeLockArgsBuilder::default()
            .admin_identity(
                generated::Identity::try_from(admin_identity)
                    .map_err(|e| CoreError::DecodeHexError(e))?,
            )
            .type_id_hash(Default::default())
            .build();

        let lock_script = packed::ScriptBuilder::default()
            .args(lock_args.as_bytes().pack())
            .build();

        let stake_infos = stake_config
            .stake_infos
            .iter()
            .map(|info| info.clone().try_into())
            .collect::<Result<Vec<generated::StakeInfo>, _>>()
            .map_err(|e| CoreError::DecodeHexError(e))?;
        let data = generated::StakeLockCellDataBuilder::default()
            .version(packed::Byte::new(stake_config.version))
            .quorum_size(packed::Byte::new(stake_config.quoram_size))
            .stake_infos(
                generated::StakeInfoVecBuilder::default()
                    .extend(stake_infos.into_iter())
                    .build(),
            )
            .sudt_type_hash(Default::default())
            .build()
            .as_bytes();

        Ok((
            packed::CellOutputBuilder::default()
                .type_(Some(type_script).pack())
                .lock(lock_script)
                .build_exact_capacity(Capacity::shannons(data.len() as u64 * BYTE_SHANNONS))
                .unwrap(),
            data,
        ))
    }

    pub(crate) fn build_omni_cell(
        &self,
        omni_config: OmniConfig,
        admin_identity: Identity,
    ) -> InnerResult<(packed::CellOutput, Bytes)> {
        let type_script = packed::ScriptBuilder::default()
            .args(H256::default().0.to_vec().pack())
            .build();

        let lock_args = generated::OmniLockArgsBuilder::default()
            .identity(
                generated::Identity::try_from(admin_identity)
                    .map_err(|e| CoreError::DecodeHexError(e))?,
            )
            .flag(packed::Byte::new(8))
            .omni_type_hash(Default::default())
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
            .max_supply(pack_u128(omni_config.max_supply.parse().unwrap()))
            .build()
            .as_bytes();

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(Some(type_script).pack())
                .build_exact_capacity(Capacity::shannons(data.len() as u64 * BYTE_SHANNONS))
                .unwrap(),
            data,
        ))
    }

    pub(crate) fn build_selection_cell(
        &self,
        checkpoint_lock_hash: H256,
    ) -> InnerResult<packed::CellOutput> {
        let lock_script = self
            .builtin_scripts
            .get(AXON_SELECTION_LOCK)
            .ok_or_else(|| CoreError::MissingAxonCellInfo(TYPE_ID_SCRIPT.to_string()))?
            .script
            .clone()
            .as_builder()
            .args(
                generated::SelectionLockArgsBuilder::default()
                    .omni_lock_hash(Default::default())
                    .checkpoint_lock_hash(checkpoint_lock_hash.pack().into())
                    .build()
                    .as_bytes()
                    .pack(),
            )
            .build();

        Ok(packed::CellOutputBuilder::default()
            .lock(lock_script)
            .build_exact_capacity(Capacity::zero())
            .unwrap())
    }

    pub(crate) fn build_checkpoint_cell(
        &self,
        mut checkpoint_config: CheckpointConfig,
        admin_identity: Identity,
    ) -> InnerResult<(packed::CellOutput, Bytes)> {
        let type_script = packed::ScriptBuilder::default()
            .args(H256::default().0.to_vec().pack())
            .build();

        let lock_script = self
            .builtin_scripts
            .get(AXON_CHECKPOINT_LOCK)
            .ok_or_else(|| CoreError::MissingAxonCellInfo(AXON_CHECKPOINT_LOCK.to_string()))?
            .script
            .clone()
            .as_builder()
            .args(
                generated::Identity::try_from(admin_identity)
                    .map_err(CoreError::DecodeHexError)?
                    .as_bytes()
                    .pack(),
            )
            .build();

        let common_ref = hex::decode(&checkpoint_config.common_ref.split_off(2)).unwrap();

        let data = generated::CheckpointLockCellDataBuilder::default()
            .version(packed::Byte::new(checkpoint_config.version))
            .state(packed::Byte::new(0))
            .period(pack_u64(0))
            .era(pack_u64(0))
            .block_hash(Default::default())
            .period_interval(pack_u32(checkpoint_config.period_intervial))
            .era_period(pack_u32(checkpoint_config.era_period))
            .unlock_period(pack_u32(0))
            .base_reward(pack_u128(checkpoint_config.base_reward.parse().unwrap()))
            .half_period(pack_u64(checkpoint_config.half_period))
            .common_ref(
                generated::Byte10Builder::default()
                    .set(to_packed_array::<10>(&common_ref))
                    .build(),
            )
            .sudt_type_hash(Default::default())
            .stake_type_hash(Default::default())
            .withdrawal_lock_code_hash(checkpoint_config.withdrawal_lock_hash.pack().into())
            .build()
            .as_bytes();

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(Some(type_script).pack())
                .build_exact_capacity(Capacity::shannons(data.len() as u64 * BYTE_SHANNONS))
                .unwrap(),
            data,
        ))
    }

    pub(crate) fn build_sudt_script(&self, args: packed::Byte32) -> packed::Script {
        self.builtin_scripts
            .get(SUDT)
            .cloned()
            .unwrap()
            .script
            .as_builder()
            .args(args.raw_data().pack())
            .build()
    }

    pub(crate) fn build_acp_cell(&self, args: Bytes) -> packed::Script {
        self.builtin_scripts
            .get(ACP)
            .cloned()
            .unwrap()
            .script
            .as_builder()
            .args(args.pack())
            .build()
    }

    pub(crate) fn build_type_id_script(
        &self,
        first_input_out_point: &packed::OutPoint,
        cell_index: u32,
    ) -> InnerResult<packed::Script> {
        let mut tmp = first_input_out_point.as_bytes().to_vec();
        tmp.extend_from_slice(&cell_index.to_le_bytes());
        let args = blake2b_256(&tmp).to_vec();

        Ok(packed::ScriptBuilder::default()
            .code_hash(TYPE_ID_CODE_HASH.pack())
            .args(args.pack())
            .build())
    }
}
