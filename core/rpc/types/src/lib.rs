#[allow(dead_code)]
pub mod consts;
pub mod error;
pub mod indexer;
pub mod lazy;

use crate::error::TypeError;

use ckb_jsonrpc_types::{
    BlockNumber, CellDep, CellOutput, OutPoint, Script, TransactionView, TransactionWithStatus,
    Uint128, Uint32, Uint64,
};
use ckb_types::{bytes::Bytes, H160, H256};
use common::{derive_more::Display, utils::to_fixed_array, NetworkType, Order, Result};
use protocol::db::TransactionWrapper;
use serde::{Deserialize, Serialize};

use std::cmp::Ordering;
use std::collections::HashSet;

pub const SECP256K1_WITNESS_LOCATION: (u32, u32) = (20, 65); // (offset, length)

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
    pub reward: Uint64,
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
        DaoInfo {
            state,
            reward: reward.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionInfo {
    pub tx_hash: H256,
    pub records: Vec<Record>,
    pub fee: Uint64,
    pub burn: Vec<BurnInfo>,
    pub timestamp: Uint64,
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
                timestamp: Some(tx.timestamp.into()),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TxRichStatus {
    pub status: ckb_jsonrpc_types::Status,
    pub block_hash: Option<H256>,
    pub reason: Option<String>,
    pub timestamp: Option<Uint64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Record {
    pub out_point: OutPoint,
    pub io_type: IOType,
    pub amount: Uint128,
    pub occupied: Uint64,
    pub asset_info: AssetInfo,
    pub extra: Option<ExtraFilter>,
    pub block_number: BlockNumber,
    pub epoch_number: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BurnInfo {
    pub udt_hash: H256,
    pub amount: Uint128,
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
    pub asset_info: AssetInfo,
    pub free: Uint128,
    pub occupied: Uint128,
    pub frozen: Uint128,
}

impl Balance {
    pub fn new(asset_info: AssetInfo) -> Self {
        Balance {
            asset_info,
            free: 0u128.into(),
            occupied: 0u128.into(),
            frozen: 0u128.into(),
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
    pub timestamp: Uint64,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetTransactionInfoResponse {
    pub transaction: Option<TransactionInfo>,
    pub status: TransactionStatus,
    pub reject_reason: Option<Uint32>,
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
    pub account_number: Uint32,
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
    pub account_number: Option<Uint32>,
    pub extra_ckb: Option<Uint64>,
    pub fee_rate: Option<Uint64>,
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
    pub fee_rate: Option<Uint64>,
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
    pub fn get_signature_offset(&self) -> (u32, u32) {
        match *self {
            SignAlgorithm::Secp256k1 => SECP256K1_WITNESS_LOCATION,
            SignAlgorithm::EthereumPersonal => SECP256K1_WITNESS_LOCATION,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureLocation {
    pub index: Uint32,  // The index in witensses vector
    pub offset: Uint32, // The start byte offset in witness encoded bytes
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
    pub other_indexes_in_group: Vec<Uint32>,
}

impl SignatureAction {
    pub fn add_group(&mut self, input_index: u32) {
        self.other_indexes_in_group.push(input_index.into())
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
    pub fee_rate: Option<Uint64>,
    pub since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct From {
    pub items: Vec<JsonItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct To {
    pub to_infos: Vec<ToInfo>,
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ToInfo {
    pub address: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SinceConfig {
    pub flag: SinceFlag,
    pub type_: SinceType,
    pub value: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SimpleTransferPayload {
    pub asset_info: AssetInfo,
    pub from: Vec<String>,
    pub to: Vec<ToInfo>,
    pub change: Option<String>,
    pub fee_rate: Option<Uint64>,
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
    pub amount: Uint64,
    pub fee_rate: Option<Uint64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoWithdrawPayload {
    pub from: JsonItem,
    pub pay_fee: Option<String>,
    pub fee_rate: Option<Uint64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoClaimPayload {
    pub from: JsonItem,
    pub to: Option<String>,
    pub fee_rate: Option<Uint64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetSpentTransactionPayload {
    pub outpoint: OutPoint,
    pub structure_type: StructureType,
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
#[serde(tag = "type", content = "value")]
pub enum SyncState {
    ReadOnly,
    ParallelFirstStage(SyncProgress),
    ParallelSecondStage(SyncProgress),
    Serial(SyncProgress),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SyncProgress {
    pub current: String,
    pub target: String,
    pub progress: String,
}

impl SyncProgress {
    pub fn new(current: u64, target: u64, progress: String) -> Self {
        SyncProgress {
            current: current.to_string(),
            target: target.to_string(),
            progress,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationRequest {
    pub cursor: Option<Uint64>,
    pub order: Order,
    pub limit: Option<Uint64>,
    pub skip: Option<Uint64>,
    pub return_count: bool,
}

impl PaginationRequest {
    pub fn new(
        cursor: Option<u64>,
        order: Order,
        limit: Option<u64>,
        skip: Option<u64>,
        return_count: bool,
    ) -> PaginationRequest {
        PaginationRequest {
            cursor: cursor.map(Into::into),
            order,
            limit: limit.map(Into::into),
            skip: skip.map(Into::into),
            return_count,
        }
    }
}

impl std::convert::From<PaginationRequest> for common::PaginationRequest {
    fn from(page: PaginationRequest) -> Self {
        common::PaginationRequest {
            cursor: page.cursor.map(Into::into),
            order: page.order,
            limit: page.limit.map(Into::into),
            skip: page.skip.map(Into::into),
            return_count: page.return_count,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationResponse<T> {
    pub response: Vec<T>,
    pub next_cursor: Option<Uint64>,
    pub count: Option<Uint64>,
}

impl<T> PaginationResponse<T> {
    pub fn new(response: Vec<T>) -> Self {
        Self {
            response,
            next_cursor: None,
            count: None,
        }
    }
}

impl<T> Default for PaginationResponse<T> {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "range from {} to {}", from, to)]
pub struct Range {
    pub from: Uint64,
    pub to: Uint64,
}

impl std::convert::From<Range> for common::Range {
    fn from(range: Range) -> Self {
        common::Range {
            from: range.from.into(),
            to: range.to.into(),
        }
    }
}
