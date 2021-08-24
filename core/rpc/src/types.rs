use common::{utils::to_fixed_array, Address, NetworkType, PaginationRequest, Range};

use ckb_jsonrpc_types::{
    CellDep, CellOutput, OutPoint, Script, TransactionView, TransactionWithStatus,
};
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use serde::{Deserialize, Serialize};

use std::collections::HashSet;
use std::str::FromStr;

/// RecordId is consist of out point and Address.
/// RecordId[0..32] is transaction blake256 hash.
/// RecordId[32..36] is the be_bytes of output index.
/// RecordId[36..] is the address encoded by UTF8.
pub type RecordId = Bytes;

pub fn encode_record_id(out_point: packed::OutPoint, address: Address) -> RecordId {
    let tx_hash: H256 = out_point.tx_hash().unpack();
    let mut encode = tx_hash.0.to_vec();
    let index: u32 = out_point.index().unpack();
    encode.extend_from_slice(&index.to_be_bytes());
    encode.extend_from_slice(&address.to_string().as_bytes());
    encode.into()
}

pub fn decode_record_id(id: Bytes) -> (packed::OutPoint, Address) {
    let id = id.to_vec();
    let tx_hash = H256::from_slice(&id[0..32]).unwrap();
    let index = u32::from_be_bytes(to_fixed_array::<4>(&id[32..36]));
    let addr = String::from_utf8(id[36..].to_vec()).unwrap();
    (
        packed::OutPointBuilder::default()
            .tx_hash(tx_hash.pack())
            .index(index.pack())
            .build(),
        Address::from_str(&addr).unwrap(),
    )
}

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
pub enum ExtraFilter {
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

    pub fn parse(&self) -> (IdentityFlag, H160) {
        (self.flag(), self.hash())
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
    pub extra: Option<ExtraFilter>,
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
    pub block_number: BlockNumber,
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
    pub reason: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryTransactionsPayload {
    pub item: Item,
    pub asset_types: HashSet<AssetType>,
    pub extra_filter: Option<ExtraFilter>,
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
    pub sig_entries: Vec<SignatureEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SignatureEntry {
    pub type_: WitnessType,
    pub index: usize,
    pub group_len: usize,
    pub pub_key: String,
    pub sig_type: SignatureType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransferPayload {
    pub asset_type: AssetType,
    pub from: Vec<From>,
    pub to: Vec<To>,
    pub change: Option<String>,
    pub fee_rate: Option<u64>,
    pub since: Option<SinceConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct From {
    pub item: Item,
    pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct To {
    pub address: String,
    pub mode: Mode,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SinceConfig {
    pub flag: SinceFlag,
    pub type_: SinceType,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SmartTransferPayload {
    pub asset_type: AssetType,
    pub from: Vec<String>,
    pub to: Vec<To>,
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
pub struct DepositPayload {
    pub from: Vec<From>,
    pub to: Option<String>,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct WithdrawPayload {
    pub from: Item,
    pub pay_fee: Option<Item>,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetSpentTransactionPayload {
    pub outpoint: OutPoint,
    pub view_type: ViewType,
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
    Cell(CellOutput),
    Transaction(TransactionWithStatus),
}
