pub mod indexer_legacy;

use crate::error::{InnerResult, RpcErrorMessage};

use common::{
    derive_more::Display, utils::to_fixed_array, Address, NetworkType, PaginationRequest, Range,
};

use ckb_jsonrpc_types::{
    CellDep, CellOutput, OutPoint, Script, TransactionView, TransactionWithStatus,
};
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use serde::{Deserialize, Serialize};

use ckb_types::core::EpochNumber;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::str::FromStr;

pub type JsonRecordId = String;

/// RecordId is consist of out point and Address.
/// RecordId[0..32] is transaction blake256 hash.
/// RecordId[32..36] is the be_bytes of output index.
/// RecordId[36..] is the address encoded by UTF8.
pub type RecordId = Bytes;

pub fn encode_record_id(
    out_point: packed::OutPoint,
    address_or_lock_hash: AddressOrLockHash,
) -> RecordId {
    let tx_hash: H256 = out_point.tx_hash().unpack();
    let mut encode = tx_hash.0.to_vec();
    let index: u32 = out_point.index().unpack();
    let (type_, value) = match address_or_lock_hash {
        AddressOrLockHash::Address(address) => (0u8, address),
        AddressOrLockHash::LockHash(lock_hash) => (1u8, lock_hash),
    };

    encode.extend_from_slice(&index.to_be_bytes());
    encode.extend_from_slice(&type_.to_be_bytes());
    encode.extend_from_slice(value.as_bytes());
    encode.into()
}

pub fn decode_record_id(id: Bytes) -> InnerResult<(packed::OutPoint, AddressOrLockHash)> {
    let id = id.to_vec();
    let tx_hash = H256::from_slice(&id[0..32]).unwrap();
    let index = u32::from_be_bytes(to_fixed_array::<4>(&id[32..36]));
    let type_ = u8::from_be_bytes(to_fixed_array::<1>(&id[36..37]));
    let value = String::from_utf8(id[37..].to_vec())
        .map_err(|e| RpcErrorMessage::InvalidRpcParams(e.to_string()))?;

    let outpoint = packed::OutPointBuilder::default()
        .tx_hash(tx_hash.pack())
        .index(index.pack())
        .build();
    match type_ {
        0u8 => Ok((outpoint, AddressOrLockHash::Address(value))),
        1u8 => Ok((outpoint, AddressOrLockHash::LockHash(value))),
        _ => unreachable!(),
    }
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
    CKB,
    UDT,
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "Asset type {:?} hash {:?}", asset_type, udt_hash)]
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
pub enum ExtraFilter {
    Dao(DaoInfo),
    CellBase,
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
    // first is deposit block number and last is withdraw block number
    Withdraw(BlockNumber, BlockNumber),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Item {
    Identity(Identity),
    Address(String),
    Record(RecordId),
}

impl std::convert::TryFrom<JsonItem> for Item {
    type Error = RpcErrorMessage;

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
                    return Err(RpcErrorMessage::DecodeJson(
                        "invalid identity item len".to_string(),
                    ));
                }

                let ident = hex::decode(&s).unwrap();
                Ok(Item::Identity(Identity(to_fixed_array::<21>(&ident))))
            }
            JsonItem::Record(mut s) => {
                let s = if s.starts_with("0x") {
                    s.split_off(2)
                } else {
                    s
                };

                let record =
                    hex::decode(&s).map_err(|e| RpcErrorMessage::DecodeHexError(e.to_string()))?;
                Ok(Item::Record(record.into()))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum JsonItem {
    Identity(String),
    Address(String),
    Record(String),
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
pub struct Identity(pub [u8; 21]);

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
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Record {
    pub id: JsonRecordId,
    pub address_or_lock_hash: AddressOrLockHash,
    pub amount: String,
    pub occupied: u64,
    pub asset_info: AssetInfo,
    pub status: Status,
    pub extra: Option<ExtraFilter>,
    pub block_number: BlockNumber,
    pub epoch_number: Bytes,
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
    pub address_or_lock_hash: AddressOrLockHash,
    pub asset_info: AssetInfo,
    pub free: String,
    pub occupied: String,
    pub freezed: String,
    pub claimable: String,
}

impl Balance {
    pub fn new(address_or_lock_hash: AddressOrLockHash, asset_info: AssetInfo) -> Self {
        Balance {
            address_or_lock_hash,
            asset_info,
            free: 0u128.to_string(),
            occupied: 0u128.to_string(),
            freezed: 0u128.to_string(),
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
    pub signature_entries: Vec<SignatureEntry>,
}

impl TransactionCompletionResponse {
    pub fn new(tx_view: TransactionView, signature_entries: Vec<SignatureEntry>) -> Self {
        TransactionCompletionResponse {
            tx_view,
            signature_entries,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureEntry {
    pub type_: WitnessType,
    pub index: usize,
    pub group_len: usize,
    pub pub_key: String,
    pub signature_type: SignatureType,
}

impl PartialEq for SignatureEntry {
    fn eq(&self, other: &SignatureEntry) -> bool {
        self.type_ == other.type_
            && self.pub_key == other.pub_key
            && self.signature_type == other.signature_type
    }
}

impl Eq for SignatureEntry {}

impl PartialOrd for SignatureEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignatureEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl SignatureEntry {
    pub fn add_group(&mut self) {
        self.group_len += 1;
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
pub struct SmartTransferPayload {
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
pub struct DepositPayload {
    pub from: From,
    pub to: Option<String>,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct WithdrawPayload {
    pub from: JsonItem,
    pub pay_fee: Option<JsonItem>,
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
pub enum AddressOrLockHash {
    Address(String),
    LockHash(String),
}

impl ToString for AddressOrLockHash {
    fn to_string(&self) -> String {
        match self {
            AddressOrLockHash::Address(address) => address.to_owned(),
            AddressOrLockHash::LockHash(lock_hash) => lock_hash.to_owned(),
        }
    }
}

pub struct UDTInfo {
    pub asset_info: AssetInfo,
    pub amount: u128,
}
