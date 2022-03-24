use ckb_jsonrpc_types::{OutPoint, TransactionView};
use ckb_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransferPayload {
    pub asset_info: AssetInfo,
    pub from: From,
    pub to: To,
    pub pay_fee: Option<String>,
    pub change: Option<String>,
    pub fee_rate: Option<u64>,
    pub since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SinceConfig {
    pub flag: SinceFlag,
    pub type_: SinceType,
    pub value: u64,
}

pub struct UDTInfo {
    pub asset_info: AssetInfo,
    pub amount: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct AssetInfo {
    pub asset_type: AssetType,
    pub udt_hash: H256,
}

impl AssetInfo {
    pub fn new_ckb() -> Self {
        AssetInfo::new(AssetType::CKB, H256::default())
    }

    pub fn new_udt(udt_hash: H256) -> Self {
        AssetInfo::new(AssetType::UDT, udt_hash)
    }

    fn new(asset_type: AssetType, udt_hash: H256) -> Self {
        AssetInfo {
            asset_type,
            udt_hash,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AssetType {
    CKB,
    UDT,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Mode {
    HoldByFrom,
    HoldByTo,
    PayWithAcp,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct From {
    pub items: Vec<JsonItem>,
    pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ToInfo {
    pub address: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum JsonItem {
    Identity(String),
    Address(String),
    OutPoint(OutPoint),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Source {
    Free,
    Claimable,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SinceType {
    BlockNumber,
    EpochNumber,
    Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SinceFlag {
    Relative,
    Absolute,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct To {
    pub to_infos: Vec<ToInfo>,
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionCompletionResponse {
    pub tx_view: TransactionView,
    pub signature_actions: Vec<SignatureAction>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureAction {
    pub signature_location: SignatureLocation,
    pub signature_info: SignatureInfo,
    pub hash_algorithm: HashAlgorithm,
    pub other_indexes_in_group: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureLocation {
    pub index: usize,  // The index in witensses vector
    pub offset: usize, // The start byte offset in witness encoded bytes
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureInfo {
    pub algorithm: SignAlgorithm,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum HashAlgorithm {
    Blake2b,
    Keccak256,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SignAlgorithm {
    Secp256k1,
    EthereumPersonal,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum SyncState {
    ReadOnly,
    ParallelFirstStage(SyncProgress),
    ParallelSecondStage(SyncProgress),
    Serial(SyncProgress),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SyncProgress {
    pub current: u64,
    pub target: u64,
    pub progress: String,
}
