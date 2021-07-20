use crate::{error::RpcError, rpc_impl::BYTE_SHANNONS};

use common::{anyhow::Result, MercuryError};

use ckb_jsonrpc_types::{Status as TransactionStatus, TransactionView};
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Pack, H256};
use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Serialize};

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::HashSet;

pub const SECP256K1: &str = "secp256k1_blake160";
pub const ACP: &str = "anyone_can_pay";
pub const CHEQUE: &str = "cheque";
pub const SUDT: &str = "sudt_balance";

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    PayByFrom = 0,
    LendByFrom,
    PayByTo,
}

impl Action {
    fn to_scripts(&self, is_udt: bool) -> Vec<ScriptType> {
        let pay_by_from_script = if is_udt {
            ScriptType::AnyoneCanPay
        } else {
            ScriptType::Secp256k1
        };
        match self {
            Action::PayByFrom => vec![pay_by_from_script],
            Action::LendByFrom => vec![ScriptType::Cheque],
            Action::PayByTo => vec![ScriptType::MyACP],
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Unconstrained = 0,
    Fleeting,
}

impl Source {
    fn to_scripts(&self) -> Vec<ScriptType> {
        match self {
            Source::Unconstrained => vec![ScriptType::Secp256k1, ScriptType::MyACP],
            Source::Fleeting => vec![ScriptType::ClaimableCheque],
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Unconstrained = 0,
    Fleeting,
    Locked,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignatureType {
    Secp256k1 = 0,
}

impl Default for SignatureType {
    fn default() -> Self {
        SignatureType::Secp256k1
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WitnessType {
    WitnessArgsLock,
    WitnessArgsType,
}

impl Default for WitnessType {
    fn default() -> Self {
        WitnessType::WitnessArgsLock
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ScriptType {
    Secp256k1 = 0,
    ClaimableCheque,
    Cheque,
    MyACP,
    AnyoneCanPay,
    SUDT = 5,
}

impl ScriptType {
    pub(crate) fn is_my_acp(&self) -> bool {
        self == &ScriptType::MyACP
    }

    pub(crate) fn is_acp(&self) -> bool {
        self == &ScriptType::AnyoneCanPay
    }

    pub(crate) fn is_cheque(&self) -> bool {
        self == &ScriptType::Cheque
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            ScriptType::Secp256k1 => SECP256K1,
            ScriptType::Cheque | ScriptType::ClaimableCheque => CHEQUE,
            ScriptType::MyACP | ScriptType::AnyoneCanPay => ACP,
            ScriptType::SUDT => SUDT,
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum QueryAddress {
    KeyAddress(String),
    NormalAddress(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GetBalancePayload {
    pub udt_hashes: HashSet<Option<H256>>,
    pub block_number: Option<u64>,
    pub address: QueryAddress,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetBalanceResponse {
    pub block_number: u64,
    pub balances: Vec<Balance>,
}

impl GetBalanceResponse {
    pub fn new(block_number: u64, balances: Vec<Balance>) -> Self {
        GetBalanceResponse {
            block_number,
            balances,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Balance {
    pub key_address: String,
    pub udt_hash: Option<H256>,
    pub unconstrained: String,
    pub fleeting: String,
    pub locked: String,
}

impl From<InnerBalance> for Balance {
    fn from(balance: InnerBalance) -> Self {
        Balance {
            key_address: balance.key_address,
            udt_hash: balance.udt_hash,
            unconstrained: balance.unconstrained.to_string(),
            fleeting: balance.fleeting.to_string(),
            locked: balance.locked.to_string(),
        }
    }
}

impl Balance {
    pub fn new(
        key_address: String,
        udt_hash: Option<H256>,
        unconstrained: u128,
        fleeting: u128,
        locked: u128,
    ) -> Self {
        Balance {
            key_address,
            udt_hash,
            unconstrained: unconstrained.to_string(),
            fleeting: fleeting.to_string(),
            locked: locked.to_string(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct InnerBalance {
    pub key_address: String,
    pub udt_hash: Option<H256>,
    pub unconstrained: BigUint,
    pub fleeting: BigUint,
    pub locked: BigUint,
}

impl InnerBalance {
    pub fn new(key_address: String, udt_hash: Option<H256>) -> Self {
        InnerBalance {
            key_address,
            udt_hash,
            unconstrained: 0u8.into(),
            fleeting: 0u8.into(),
            locked: 0u8.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FromAccount {
    pub idents: Vec<String>,
    pub source: Source,
}

impl FromAccount {
    pub(crate) fn to_inner(&self) -> InnerAccount {
        InnerAccount {
            idents: self.idents.clone(),
            scripts: self.source.to_scripts(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ToAccount {
    pub ident: String,
    pub action: Action,
}

impl ToAccount {
    pub(crate) fn to_inner(&self, is_udt: bool) -> InnerAccount {
        InnerAccount {
            idents: vec![self.ident.clone()],
            scripts: self.action.to_scripts(is_udt),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransferPayload {
    pub udt_hash: Option<H256>,
    pub from: FromAccount,
    pub items: Vec<TransferItem>,
    pub change: Option<String>,
    pub fee_rate: u64, // shannons/KB
}

impl TransferPayload {
    pub(crate) fn to_inner_items(&self, is_udt: bool) -> Vec<InnerTransferItem> {
        self.items
            .iter()
            .map(|item| item.to_inner(is_udt))
            .collect()
    }

    pub(crate) fn check(&self) -> Result<()> {
        if self.udt_hash.is_none()
            && self
                .items
                .iter()
                .any(|item| item.to.action != Action::PayByFrom)
        {
            return Err(MercuryError::rpc(RpcError::InvalidTransferPayload).into());
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CreateWalletPayload {
    pub ident: String,
    pub info: Vec<WalletInfo>,
    pub fee_rate: u64, // shannons/KB
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct WalletInfo {
    pub udt_hash: H256,
    pub min_ckb: Option<u8>,
    pub min_udt: Option<u8>,
}

impl WalletInfo {
    pub fn check(&self) -> Result<()> {
        if self.min_udt.is_some() && self.min_ckb.is_none() {
            return Err(MercuryError::rpc(RpcError::InvalidAccountInfo).into());
        }

        Ok(())
    }

    pub fn expected_capacity(&self) -> u64 {
        let mut ret = 142u64;

        if self.min_ckb.is_some() {
            ret += 1;
        }

        if self.min_udt.is_some() {
            ret += 1;
        }

        ret * BYTE_SHANNONS
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransferItem {
    pub to: ToAccount,
    pub amount: u128,
}

impl TransferItem {
    pub(crate) fn to_inner(&self, is_udt: bool) -> InnerTransferItem {
        InnerTransferItem {
            to: self.to.to_inner(is_udt),
            amount: self.amount,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TransactionCompletionResponse {
    pub tx_view: TransactionView,
    pub sigs_entry: Vec<SignatureEntry>,
}

impl TransactionCompletionResponse {
    pub fn new(tx_view: TransactionView, sigs_entry: Vec<SignatureEntry>) -> Self {
        TransactionCompletionResponse {
            tx_view,
            sigs_entry,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SignatureEntry {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub type_: WitnessType,
    pub index: usize,
    pub group_len: usize,
    pub pub_key: String,
    pub sig_type: SignatureType,
}

impl PartialEq for SignatureEntry {
    fn eq(&self, other: &SignatureEntry) -> bool {
        self.type_ == other.type_
            && self.pub_key == other.pub_key
            && self.sig_type == other.sig_type
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
    pub fn new(index: usize, pub_key: String, sig_type: SignatureType) -> Self {
        SignatureEntry {
            type_: WitnessType::WitnessArgsLock,
            group_len: 1,
            pub_key,
            index,
            sig_type,
        }
    }

    pub fn add_group(&mut self) {
        self.group_len += 1;
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct InnerAccount {
    pub(crate) idents: Vec<String>,
    pub(crate) scripts: Vec<ScriptType>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct InnerTransferItem {
    pub(crate) to: InnerAccount,
    pub(crate) amount: u128,
}

#[derive(Default, Clone, Debug)]
pub struct CellWithData {
    pub cell: packed::CellOutput,
    pub data: packed::Bytes,
}

impl CellWithData {
    pub fn new(cell: packed::CellOutput, data: Bytes) -> Self {
        CellWithData {
            cell,
            data: data.pack(),
        }
    }
}

// Todo: only remain ckb_all and udt_amount
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DetailedAmount {
    pub udt_amount: u128,
    pub ckb_all: u64,
}

impl DetailedAmount {
    pub fn new() -> Self {
        DetailedAmount::default()
    }

    pub fn add_udt_amount(&mut self, amount: u128) {
        self.udt_amount += amount;
    }

    pub fn add_ckb_all(&mut self, amount: u64) {
        self.ckb_all += amount;
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct InputConsume {
    pub ckb: u64,
    pub udt: u128,
}

impl InputConsume {
    pub fn new(ckb: u64, udt: u128) -> Self {
        InputConsume { ckb, udt }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Operation {
    pub id: u32,
    pub key_address: String,
    pub normal_address: String,
    pub amount: Amount,
}

impl Operation {
    pub fn new(id: u32, key_address: String, normal_address: String, amount: Amount) -> Self {
        Operation {
            id,
            key_address,
            normal_address,
            amount,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Amount {
    pub value: String,
    pub udt_hash: Option<H256>,
    pub status: Status,
}

impl From<InnerAmount> for Amount {
    fn from(inner: InnerAmount) -> Self {
        Amount {
            value: inner.value.to_string(),
            udt_hash: inner.udt_hash,
            status: inner.status,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct InnerAmount {
    pub value: BigInt,
    pub udt_hash: Option<H256>,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetGenericBlockPayload {
    pub block_num: Option<u64>,
    pub block_hash: Option<H256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GenericBlock {
    block_number: BlockNumber,
    block_hash: H256,
    parent_block_hash: H256,
    timestamp: u64,
    transactions: Vec<GenericTransaction>,
}

impl GenericBlock {
    pub fn new(
        block_number: BlockNumber,
        block_hash: H256,
        parent_block_hash: H256,
        timestamp: u64,
        transactions: Vec<GenericTransaction>,
    ) -> Self {
        GenericBlock {
            block_number,
            block_hash,
            parent_block_hash,
            timestamp,
            transactions,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GenericTransaction {
    pub tx_hash: H256,
    pub operations: Vec<Operation>,
}

impl From<GetGenericTransactionResponse> for GenericTransaction {
    fn from(tx: GetGenericTransactionResponse) -> Self {
        tx.transaction
    }
}

impl GenericTransaction {
    pub fn new(tx_hash: H256, operations: Vec<Operation>) -> Self {
        GenericTransaction {
            tx_hash,
            operations,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetGenericTransactionResponse {
    pub transaction: GenericTransaction,
    pub status: TransactionStatus,
    pub block_hash: Option<H256>,
    pub block_number: Option<BlockNumber>,
    pub confirmed_number: Option<u64>,
}

impl GetGenericTransactionResponse {
    pub fn new(
        transaction: GenericTransaction,
        status: TransactionStatus,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
        confirmed_number: Option<BlockNumber>,
    ) -> Self {
        GetGenericTransactionResponse {
            transaction,
            status,
            block_hash,
            block_number,
            confirmed_number,
        }
    }
}

pub fn details_split_off(
    detailed_cells: Vec<CellWithData>,
    outputs: &mut Vec<packed::CellOutput>,
    data_vec: &mut Vec<packed::Bytes>,
) {
    let mut cells = detailed_cells
        .iter()
        .map(|output| output.cell.clone())
        .collect::<Vec<_>>();
    let mut data = detailed_cells
        .into_iter()
        .map(|output| output.data)
        .collect::<Vec<_>>();

    outputs.append(&mut cells);
    data_vec.append(&mut data);
}

#[cfg(test)]
mod tests {
    use super::*;

    use core_extensions::{special_cells, udt_balance};

    #[test]
    fn test_constant_eq() {
        assert_eq!(ACP, special_cells::ACP);
        assert_eq!(CHEQUE, special_cells::CHEQUE);
        assert_eq!(SUDT, udt_balance::SUDT)
    }
}
