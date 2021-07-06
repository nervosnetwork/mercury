use crate::{error::RpcError, rpc_impl::BYTE_SHANNONS};

use common::{anyhow::Result, MercuryError};

use ckb_jsonrpc_types::TransactionView;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Pack, H256};
use serde::{Deserialize, Serialize};

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

pub const SECP256K1: &str = "secp256k1_blake160";
pub const ACP: &str = "anyone_can_pay";
pub const CHEQUE: &str = "cheque";
pub const SUDT: &str = "sudt_balance";

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
#[derive(Serialize, Deserialize, Clone, Debug)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetBalanceResponse {
    pub unconstrained: String,
    pub fleeting: String,
    pub locked: String,
}

impl GetBalanceResponse {
    pub fn new(unconstrained: u128, fleeting: u128, locked: u128) -> Self {
        GetBalanceResponse {
            unconstrained: unconstrained.to_string(),
            fleeting: fleeting.to_string(),
            locked: locked.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateWalletPayload {
    pub ident: String,
    pub info: Vec<WalletInfo>,
    pub fee_rate: u64, // shannons/KB
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanBlockPayload {
    pub block_number: BlockNumber,
    pub udt_hash: Option<H256>,
    pub idents: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanBlockResponse {
    pub inner: Vec<JsonCharge>,
}

impl ScanBlockResponse {
    pub fn new(inner: Vec<InnerCharge>) -> Self {
        ScanBlockResponse {
            inner: inner.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonCharge {
    pub address: String,
    pub ckb_amount: String,
    pub udt_amount: String,
}

impl From<InnerCharge> for JsonCharge {
    fn from(input: InnerCharge) -> Self {
        JsonCharge {
            address: input.address,
            ckb_amount: input.ckb_amount.to_string(),
            udt_amount: input.udt_amount.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InnerCharge {
    pub address: String,
    pub ckb_amount: u64,
    pub udt_amount: u128,
}

impl InnerCharge {
    pub fn new(addr: String) -> Self {
        InnerCharge {
            address: addr,
            ckb_amount: 0,
            udt_amount: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
}

impl PartialEq for SignatureEntry {
    fn eq(&self, other: &SignatureEntry) -> bool {
        self.type_ == other.type_ && self.pub_key == other.pub_key
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
    pub fn new(index: usize, pub_key: String) -> Self {
        SignatureEntry {
            type_: WitnessType::WitnessArgsLock,
            group_len: 1,
            pub_key,
            index,
        }
    }

    pub fn add_group(&mut self) {
        self.group_len += 1;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InnerAccount {
    pub(crate) idents: Vec<String>,
    pub(crate) scripts: Vec<ScriptType>,
}

#[derive(Clone, Debug)]
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
#[derive(Default, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub struct InputConsume {
    pub ckb: u64,
    pub udt: u128,
}

impl InputConsume {
    pub fn new(ckb: u64, udt: u128) -> Self {
        InputConsume { ckb, udt }
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
