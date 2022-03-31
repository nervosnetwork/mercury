#[allow(dead_code)]
pub mod consts;
pub mod error;
pub mod indexer;
pub mod lazy;

use crate::error::TypeError;

use common::{
    derive_more::Display, utils::to_fixed_array, NetworkType, PaginationRequest, Range, Result,
};
use protocol::db::TransactionWrapper;

use ckb_jsonrpc_types::{
    CellDep, CellOutput, OutPoint, Script, TransactionView, TransactionWithStatus,
};
use ckb_types::{bytes::Bytes, core::BlockNumber, H160, H256};
use serde::{Deserialize, Serialize};

use std::cmp::Ordering;
use std::collections::HashSet;

/// RecordId is consist of out point and Address.
/// RecordId[0..32] is transaction blake256 hash.
/// RecordId[32..36] is the be_bytes of output index.
/// RecordId[36..] is the address encoded by UTF8.
pub type RecordId = Bytes;

pub const SECP256K1_WITNESS_LOCATION: (usize, usize) = (20, 65); // (offset, length)

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum Status {
    Claimable(BlockNumber),
    Fixed(BlockNumber),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Source {
    Free,
    Claimable,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AssetType {
    CKB,
    UDT,
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "Asset type {:?} hash {}", asset_type, udt_hash)]
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
#[serde(tag = "type", content = "value")]
pub enum ExtraFilter {
    Dao(DaoInfo),
    CellBase,
    /// Cell data or type is not empty, except Dao and Acp UDT cell.
    /// This is an important mark for accumulate_balance.
    Freeze,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum ExtraType {
    Dao,
    CellBase,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum StructureType {
    Native,
    DoubleEntry,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum IOType {
    Input,
    Output,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum QueryType {
    Cell,
    Transaction,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum TxView {
    TransactionWithRichStatus(TransactionWithRichStatus),
    TransactionInfo(TransactionInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum DaoState {
    Deposit(BlockNumber),
    // first is deposit block number and last is withdraw block number
    Withdraw(BlockNumber, BlockNumber),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Item {
    Identity(Identity),
    Address(String),
    OutPoint(OutPoint),
}

impl std::convert::TryFrom<JsonItem> for Item {
    type Error = TypeError;

    fn try_from(json_item: JsonItem) -> Result<Self, Self::Error> {
        match json_item {
            JsonItem::Address(s) => Ok(Item::Address(s)),
            JsonItem::Identity(mut s) => {
                let s = if s.starts_with("0x") {
                    s.split_off(2)
                } else {
                    s
                };

                if s.len() != 42 {
                    return Err(TypeError::DecodeJson(
                        "invalid identity item len".to_string(),
                    ));
                }

                let ident = hex::decode(&s).unwrap();
                Ok(Item::Identity(Identity(to_fixed_array::<21>(&ident))))
            }
            JsonItem::OutPoint(out_point) => Ok(Item::OutPoint(out_point)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum JsonItem {
    Identity(String),
    Address(String),
    OutPoint(OutPoint),
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TransactionStatus {
    pending,
    proposed,
    committed,
    rejected,
    unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Mode {
    HoldByFrom,
    HoldByTo,
    PayWithAcp,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SinceFlag {
    Relative,
    Absolute,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SinceType {
    BlockNumber,
    EpochNumber,
    Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum IdentityFlag {
    Ckb = 0x0,
    Ethereum = 0x1,
    Eos = 0x2,
    Tron = 0x3,
    Bitcoin = 0x4,
    Dogecoin = 0x5,
    OwnerLock = 0xFC,
    Exec = 0xFD,
    DI = 0xFE,
}

impl TryFrom<u8> for IdentityFlag {
    type Error = TypeError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        let ret = match v {
            0x0 => IdentityFlag::Ckb,
            0x1 => IdentityFlag::Ethereum,
            0x2 => IdentityFlag::Eos,
            0x3 => IdentityFlag::Tron,
            0x4 => IdentityFlag::Bitcoin,
            0x5 => IdentityFlag::Dogecoin,
            0xFC => IdentityFlag::OwnerLock,
            0xFD => IdentityFlag::Exec,
            0xFE => IdentityFlag::DI,
            _ => return Err(TypeError::UnsupportIdentityFlag(v)),
        };

        Ok(ret)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identity(pub [u8; 21]);

impl Identity {
    pub fn new(flag: IdentityFlag, hash: H160) -> Self {
        let mut inner = vec![flag as u8];
        inner.extend_from_slice(&hash.0);
        Identity(to_fixed_array::<21>(&inner))
    }

    pub fn parse(&self) -> Result<(IdentityFlag, H160), TypeError> {
        Ok((self.flag()?, self.hash()))
    }

    pub fn flag(&self) -> Result<IdentityFlag, TypeError> {
        let ret = self.0[0].try_into()?;
        Ok(ret)
    }

    pub fn hash(&self) -> H160 {
        H160::from_slice(&self.0[1..21]).unwrap()
    }

    pub fn encode(&self) -> String {
        let mut identity_string: String = "0x".to_owned();
        identity_string.push_str(&hex::encode(self.0));
        identity_string
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoInfo {
    pub state: DaoState,
    pub reward: u64,
}

impl DaoInfo {
    pub fn new_withdraw(
        deposit_block_number: BlockNumber,
        withdraw_block_number: BlockNumber,
        reward: u64,
    ) -> Self {
        DaoInfo::new(
            DaoState::Withdraw(deposit_block_number, withdraw_block_number),
            reward,
        )
    }

    pub fn new_deposit(block_number: BlockNumber, reward: u64) -> Self {
        DaoInfo::new(DaoState::Deposit(block_number), reward)
    }

    fn new(state: DaoState, reward: u64) -> Self {
        DaoInfo { state, reward }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionInfo {
    pub tx_hash: H256,
    pub records: Vec<Record>,
    pub fee: u64,
    pub burn: Vec<BurnInfo>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionWithRichStatus {
    pub transaction: Option<TransactionView>,
    pub tx_status: TxRichStatus,
}

impl std::convert::From<TransactionWrapper> for TransactionWithRichStatus {
    fn from(tx: TransactionWrapper) -> Self {
        TransactionWithRichStatus {
            transaction: tx.transaction_with_status.transaction,
            tx_status: TxRichStatus {
                status: tx.transaction_with_status.tx_status.status,
                block_hash: tx.transaction_with_status.tx_status.block_hash,
                reason: tx.transaction_with_status.tx_status.reason,
                timestamp: Some(tx.timestamp),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TxRichStatus {
    pub status: ckb_jsonrpc_types::Status,
    pub block_hash: Option<H256>,
    pub reason: Option<String>,
    pub timestamp: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Record {
    pub out_point: OutPoint,
    pub ownership: Ownership,
    pub amount: String,
    pub occupied: u64,
    pub asset_info: AssetInfo,
    pub status: Status,
    pub extra: Option<ExtraFilter>,
    pub block_number: BlockNumber,
    pub epoch_number: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BurnInfo {
    pub udt_hash: H256,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetBalancePayload {
    pub item: JsonItem,
    pub asset_infos: HashSet<AssetInfo>,
    pub tip_block_number: Option<BlockNumber>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetBalanceResponse {
    pub balances: Vec<Balance>,
    pub tip_block_number: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Balance {
    pub ownership: Ownership,
    pub asset_info: AssetInfo,
    pub free: String,
    pub occupied: String,
    pub frozen: String,
    pub claimable: String,
}

impl Balance {
    pub fn new(ownership: Ownership, asset_info: AssetInfo) -> Self {
        Balance {
            ownership,
            asset_info,
            free: 0u128.to_string(),
            occupied: 0u128.to_string(),
            frozen: 0u128.to_string(),
            claimable: 0u128.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetBlockInfoPayload {
    pub block_number: Option<BlockNumber>,
    pub block_hash: Option<H256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BlockInfo {
    pub block_number: BlockNumber,
    pub block_hash: H256,
    pub parent_hash: H256,
    pub timestamp: u64,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetTransactionInfoResponse {
    pub transaction: Option<TransactionInfo>,
    pub status: TransactionStatus,
    pub reject_reason: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryTransactionsPayload {
    pub item: JsonItem,
    pub asset_infos: HashSet<AssetInfo>,
    pub extra: Option<ExtraType>,
    pub block_range: Option<Range>,
    pub pagination: PaginationRequest,
    pub structure_type: StructureType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetAccountInfoPayload {
    pub item: JsonItem,
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetAccountInfoResponse {
    pub account_number: u32,
    pub account_address: String,
    pub account_type: AccountType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AccountType {
    Acp,
    PwLock,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AdjustAccountPayload {
    pub item: JsonItem,
    pub from: HashSet<JsonItem>,
    pub asset_info: AssetInfo,
    pub account_number: Option<u32>,
    pub extra_ckb: Option<u64>,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransactionCompletionResponse {
    pub tx_view: TransactionView,
    pub signature_actions: Vec<SignatureAction>,
}

impl TransactionCompletionResponse {
    pub fn new(tx_view: TransactionView, signature_actions: Vec<SignatureAction>) -> Self {
        TransactionCompletionResponse {
            tx_view,
            signature_actions,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SudtIssuePayload {
    pub owner: String,
    pub to: To,
    pub pay_fee: Option<JsonItem>,
    pub change: Option<String>,
    pub fee_rate: Option<u64>,
    pub since: Option<SinceConfig>,
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

impl SignAlgorithm {
    pub fn get_signature_offset(&self) -> (usize, usize) {
        match *self {
            SignAlgorithm::Secp256k1 => SECP256K1_WITNESS_LOCATION,
            SignAlgorithm::EthereumPersonal => SECP256K1_WITNESS_LOCATION,
        }
    }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureAction {
    pub signature_location: SignatureLocation,
    pub signature_info: SignatureInfo,
    pub hash_algorithm: HashAlgorithm,
    pub other_indexes_in_group: Vec<usize>,
}

impl SignatureAction {
    pub fn add_group(&mut self, input_index: usize) {
        self.other_indexes_in_group.push(input_index)
    }
}

impl PartialEq for SignatureAction {
    fn eq(&self, other: &SignatureAction) -> bool {
        self.signature_info.address == other.signature_info.address
            && self.signature_info.algorithm == other.signature_info.algorithm
    }
}

impl Eq for SignatureAction {}

impl PartialOrd for SignatureAction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignatureAction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.signature_location
            .index
            .cmp(&other.signature_location.index)
    }
}

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
pub struct From {
    pub items: Vec<JsonItem>,
    pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct To {
    pub to_infos: Vec<ToInfo>,
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ToInfo {
    pub address: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SinceConfig {
    pub flag: SinceFlag,
    pub type_: SinceType,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SimpleTransferPayload {
    pub asset_info: AssetInfo,
    pub from: Vec<String>,
    pub to: Vec<ToInfo>,
    pub change: Option<String>,
    pub fee_rate: Option<u64>,
    pub since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct MercuryInfo {
    pub mercury_version: String,
    pub ckb_node_version: String,
    pub network_type: NetworkType,
    pub enabled_extensions: Vec<Extension>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Extension {
    pub name: String,
    pub scripts: Vec<Script>,
    pub cell_deps: Vec<CellDep>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoDepositPayload {
    pub from: From,
    pub to: Option<String>,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoWithdrawPayload {
    pub from: JsonItem,
    pub pay_fee: Option<String>,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoClaimPayload {
    pub from: JsonItem,
    pub to: Option<String>,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetSpentTransactionPayload {
    pub outpoint: OutPoint,
    pub structure_type: StructureType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct AdvanceQueryPayload {
    pub lock: Option<ScriptWrapper>,
    pub type_: Option<ScriptWrapper>,
    pub data: Option<String>,
    pub args_len: Option<u32>,
    pub block_range: Option<Range>,
    pub pagination: PaginationRequest,
    pub query_type: QueryType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ScriptWrapper {
    pub script: Option<Script>,
    pub io_type: Option<IOType>,
    pub args_len: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum QueryResponse {
    Cell(CellInfo),
    Transaction(TransactionWithStatus),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CellInfo {
    cell_output: CellOutput,
    out_point: OutPoint,
    block_hash: H256,
    block_number: BlockNumber,
    data: Bytes,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct RequiredUDT {
    pub udt_hash: H256,
    pub amount_required: i128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum Ownership {
    Address(String),
    LockHash(String),
}

impl ToString for Ownership {
    fn to_string(&self) -> String {
        match self {
            Ownership::Address(address) => address.to_owned(),
            Ownership::LockHash(lock_hash) => lock_hash.to_owned(),
        }
    }
}

pub struct UDTInfo {
    pub asset_info: AssetInfo,
    pub amount: u128,
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

impl SyncProgress {
    pub fn new(current: u64, target: u64, progress: String) -> Self {
        SyncProgress {
            current,
            target,
            progress,
        }
    }
}
