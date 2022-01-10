pub mod generated;

use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

pub const AXON_CHECKPOINT_LOCK: &str = "axon_checkpoint_lock";
pub const AXON_SELECTION_LOCK: &str = "axon_selection_lock";

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identity {
    pub flag: u8,
    pub content: Bytes,
}

impl TryFrom<Identity> for generated::Identity {
    type Error = String;

    fn try_from(id: Identity) -> Result<Self, Self::Error> {
        if id.content.len() != 20 {
            return Err(String::from("Invalid Admin Identity"));
        }

        let content = hex::decode(id.content.clone().split_off(2)).map_err(|e| e.to_string())?;

        Ok(generated::IdentityBuilder::default()
            .flag(packed::Byte::new(id.flag))
            .content(
                generated::Byte20Builder::default()
                    .set(to_packed_array::<20>(&content))
                    .build(),
            )
            .build())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct OmniConfig {
    pub version: u8,
    pub max_supply: BigUint,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CheckpointConfig {
    pub version: u8,
    pub period_intervial: u32,
    pub era_period: u32,
    pub base_reward: BigUint,
    pub half_period: u64,
    pub common_ref: Bytes,
    pub withdrawal_lock_hash: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StakeInfo {
    pub identity: Identity,
    pub l2_address: H160,
    pub bls_pub_key: Bytes,
    pub stake_amount: BigUint,
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
    pub udt_hash: H256,
    pub omni_type_hash: H256,
    pub checkpoint_type_hash: H256,
    pub stake_type_hash: H256,
    pub selection_lock_hash: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct InitChainPayload {
    pub omni_config: OmniConfig,
    pub check_point_config: CheckpointConfig,
    pub state_config: StakeConfig,
    pub admin_id: Identity,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct IssueAssetPayload {
    pub admin_id: Identity,
    pub selection_lock_hash: H256,
    pub omni_type_hash: H256,
    pub receipt_address: Bytes,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SubmitCheckPointPayload {
    pub node_id: Identity,
    pub admin_id: Identity,
    pub check_point: Bytes,
    pub selection_lock_hash: H256,
    pub checkpoint_type_hash: H256,
}

pub fn to_packed_array<const LEN: usize>(input: &[u8]) -> [packed::Byte; LEN] {
    assert_eq!(input.len(), LEN);
    let mut list = [packed::Byte::new(0); LEN];
    for (idx, item) in list.iter_mut().enumerate() {
        *item = packed::Byte::new(input[idx]);
    }
    list
}

impl From<packed::Byte32> for generated::Byte32 {
    fn from(byte32: packed::Byte32) -> Self {
        generated::Byte32::new_unchecked(byte32.as_bytes())
    }
}

pub fn pack_u32(input: u32) -> generated::Byte4 {
    generated::Byte4Builder::default()
        .set(to_packed_array::<4>(&input.to_le_bytes()))
        .build()
}

pub fn pack_u64(input: u64) -> generated::Byte8 {
    generated::Byte8Builder::default()
        .set(to_packed_array::<8>(&input.to_le_bytes()))
        .build()
}

pub fn pack_u128(input: u128) -> generated::Byte16 {
    generated::Byte16Builder::default()
        .set(to_packed_array::<16>(&input.to_le_bytes()))
        .build()
}
