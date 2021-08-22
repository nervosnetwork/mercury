use common::{utils::to_fixed_array, MercuryError, NetworkType, PaginationRequest, Range};

use ckb_jsonrpc_types::{
    CellDep, CellOutput, OutPoint, Script, TransactionView, TransactionWithStatus,
};
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use serde::{Deserialize, Serialize};

use std::collections::HashSet;

pub type RecordId = Bytes;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
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
    Ckb,
    UDT(H256),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Extra {
    Dao,
    CellBase,
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
pub enum ViewType {
    TransactionView,
    TransactionInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TxView {
    TransactionView(TransactionWithStatus),
    TransactionInfo(TransactionInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum WitnessType {
    WitnessLock,
    WitnessType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SignatureType {
    Secp256k1,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum DaoState {
    Deposit(BlockNumber),
    Withdraw(BlockNumber),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Item {
    Identity(Identity),
    Address(String),
    Record(RecordId),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Proposed,
    Committed,
    Rejected,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Mode {
    HoldByFrom,
    HoldByTo,
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

impl std::convert::From<u8> for IdentityFlag {
    fn from(v: u8) -> Self {
        match v {
            0x0 => IdentityFlag::Ckb,
            0x1 => IdentityFlag::Ethereum,
            0x2 => IdentityFlag::Eos,
            0x3 => IdentityFlag::Tron,
            0x4 => IdentityFlag::Bitcoin,
            0x5 => IdentityFlag::Dogecoin,
            0xFC => IdentityFlag::OwnerLock,
            0xFD => IdentityFlag::Exec,
            0xFE => IdentityFlag::DI,
            _ => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identity([u8; 21]);

impl Identity {
    pub fn new(flag: IdentityFlag, hash: H160) -> Self {
        let mut inner = vec![flag as u8];
        inner.extend_from_slice(&hash.0);
        Identity(to_fixed_array::<21>(&inner))
    }

    pub fn flag(&self) -> IdentityFlag {
        self.0[0].into()
    }

    pub fn hash(&self) -> H160 {
        H160::from_slice(&self.0[1..21]).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DaoInfo {
    pub state: DaoState,
    pub reward: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionInfo {
    pub tx_hash: H256,
    pub records: Vec<Record>,
    pub fee: u64,
    pub brun: Vec<BurnInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Record {
    pub id: RecordId,
    pub address: String,
    pub amount: u128,
    pub asset_type: AssetType,
    pub status: Status,
    pub extra: Option<Extra>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BurnInfo {
    pub udt_hash: H256,
    pub amount: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetBalancePayload {
    pub item: Item,
    pub asset_types: HashSet<AssetType>,
    pub block_num: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetBalanceResponse {
    pub balances: Vec<Balance>,
    pub block_num: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Balance {
    pub address: String,
    pub asset_type: AssetType,
    pub free: String,
    pub occupied: String,
    pub feddzed: String,
    pub claimable: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetBlockInfoPayload {
    block_number: Option<BlockNumber>,
    block_hash: Option<H256>,
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
    pub reason: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryTransactionsPayload {
    pub item: Item,
    pub asset_types: HashSet<AssetType>,
    pub extra_filter: Option<Extra>,
    pub block_range: Range,
    pub pagination: PaginationRequest,
    pub view_type: ViewType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AdjustAccountPayload {
    pub item: Item,
    pub from: HashSet<Item>,
    pub asset_type: AssetType,
    pub account_number: Option<u32>,
    pub extra_ckb: Option<u64>,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionCompletionResponse {
    pub tx_view: TransactionView,
    pub sig_entrirs: Vec<SignatureEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SignatureEntry {
    type_: WitnessType,
    index: usize,
    group_len: usize,
    pub_key: String,
    sig_type: SignatureType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransferPayload {
    asset_type: AssetType,
    from: Vec<From>,
    to: Vec<To>,
    change: Option<String>,
    fee_rate: Option<u64>,
    since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct From {
    item: Item,
    source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct To {
    address: String,
    mode: Mode,
    amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SinceConfig {
    flag: SinceFlag,
    type_: SinceType,
    value: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SmartTransferPayload {
    asset_type: AssetType,
    from: Vec<String>,
    to: Vec<To>,
    change: Option<String>,
    fee_rate: Option<u64>,
    since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct MercuryInfo {
    mercury_version: String,
    ckb_node_version: String,
    network_type: NetworkType,
    enabled_extensions: Vec<Extension>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Extension {
    name: String,
    scripts: Vec<Script>,
    cell_deps: Vec<CellDep>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DepositPayload {
    from: Vec<From>,
    to: Option<String>,
    amount: u64,
    fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct WithdrawPayload {
    from: Item,
    pay_fee: Option<Item>,
    fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetSpentTransactionPayload {
    outpoint: OutPoint,
    view_type: ViewType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct AdvanceQueryPayload {
    lock: Option<ScriptWrapper>,
    type_: Option<ScriptWrapper>,
    data: Option<String>,
    args_len: Option<u32>,
    block_range: Option<Range>,
    pagination: PaginationRequest,
    query_type: QueryType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ScriptWrapper {
    script: Option<Script>,
    io_type: Option<IOType>,
    args_len: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum QueryResponse {
    Cell(CellOutput),
    Transaction(TransactionWithStatus),
}
