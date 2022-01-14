pub mod generated;

use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use common::utils::to_fixed_array;
use serde::{Deserialize, Serialize};

use crate::TransactionCompletionResponse;

pub const AXON_CHECKPOINT_LOCK: &str = "axon_checkpoint";
pub const AXON_SELECTION_LOCK: &str = "axon_selection";
pub const AXON_STAKE_LOCK: &str = "axon_stake";

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identity {
    pub flag: u8,
    pub content: Bytes,
}

impl TryFrom<Identity> for crate::Identity {
    type Error = String;

    fn try_from(id: Identity) -> Result<Self, Self::Error> {
        if id.content.len() != 42 {
            return Err(String::from("Invalid Admin Identity"));
        }

        let mut ret = vec![id.flag];
        ret.append(&mut id.content.to_vec());
        Ok(Self(to_fixed_array(&ret)))
    }
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
    pub max_supply: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CheckpointConfig {
    pub version: u8,
    pub period_intervial: u32,
    pub era_period: u32,
    pub base_reward: String,
    pub half_period: u64,
    pub common_ref: Bytes,
    pub withdrawal_lock_hash: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StakeInfo {
    pub identity: Identity,
    pub l2_address: H160,
    pub bls_pub_key: Bytes,
    pub stake_amount: String,
    pub inauguration_era: u64,
}

impl TryFrom<StakeInfo> for generated::StakeInfo {
    type Error = String;

    fn try_from(info: StakeInfo) -> Result<Self, Self::Error> {
        if info.bls_pub_key.len() != 97 {
            return Err(String::from("Invalid bls pubkey len"));
        }

        let stake_amount: u128 = info
            .stake_amount
            .clone()
            .parse()
            .map_err(|_| "stake_amount overflow".to_string())?;

        Ok(generated::StakeInfoBuilder::default()
            .identity(info.identity.try_into()?)
            .l2_address(info.l2_address.into())
            .bls_pub_key(
                generated::Byte97Builder::default()
                    .set(to_packed_array::<97>(&info.bls_pub_key))
                    .build(),
            )
            .stake_amount(pack_u128(stake_amount))
            .inauguration_era(pack_u64(info.inauguration_era))
            .build())
    }
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InitChainResponse {
    pub tx: TransactionCompletionResponse,
    pub config: SidechainConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct IssueAssetPayload {
    pub admin_id: Identity,
    pub selection_lock_hash: H256,
    pub omni_type_hash: H256,
    pub receipt_address: Bytes,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SubmitCheckPointPayload {
    pub node_id: Identity,
    pub admin_id: Identity,
    pub check_point: Bytes,
    pub selection_lock_hash: H256,
    pub checkpoint_type_hash: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CrossChainTransferPayload {
    pub relayer: H160,
    pub receiver: H160,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BuildCrossChainTransferTxPayload {
    pub sender: String,
    pub receiver: String,
    pub udt_hash: H256,
    pub amount: String,
    pub memo: String,
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

impl From<H160> for generated::Byte20 {
    fn from(h: H160) -> Self {
        generated::Byte20Builder::default()
            .set(to_packed_array::<20>(&h.0))
            .build()
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

pub fn unpack_byte16(input: generated::Byte16) -> u128 {
    let raw = input.raw_data().to_vec();
    u128::from_le_bytes(to_fixed_array(&raw))
}
