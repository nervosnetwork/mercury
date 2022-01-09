pub mod generated;

use ckb_types::{bytes::Bytes, U256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identity {
    pub flag: u8,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct OmniConfig {
    pub version: u8,
    pub max_supply: U256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CheckpointConfig {
    pub version: u8,
    pub period_intervial: u32,
    pub era_period: u32,
    pub base_reward: U256,
    pub half_period: u64,
    pub common_ref: String,
    pub withdrawal_lock_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StakeInfo {
    pub identity: Identity,
    pub l2_address: String,
    pub bls_pub_key: String,
    pub stake_amount: U256,
    pub inauguration_era: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StakeConfig {
    pub version: u8,
    pub stake_infos: Vec<StakeInfo>,
    pub quoram_size: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SidechainConfig {
    pub udt_hash: String,
    pub omni_type_hash: String,
    pub checkpoint_type_hash: String,
    pub stake_type_hash: String,
    pub selection_lock_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct InitChainPayload {
    pub omni_config: OmniConfig,
    pub check_point_config: U256,
    pub state_config: StakeConfig,
    pub admin_id: Identity,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct IssueAssetPayload {
    pub admin_id: Identity,
    pub selection_lock_hash: String,
    pub omni_type_hash: String,
    pub receipt_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SubmitCheckPointPayload {
    pub node_id: Identity,
    pub admin_id: Identity,
    pub check_point: Bytes,
    pub selection_lock_hash: String,
    pub checkpoint_type_hash: String,
}
