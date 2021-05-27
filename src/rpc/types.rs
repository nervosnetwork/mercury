use ckb_jsonrpc_types::TransactionView;
use ckb_types::{bytes::Bytes, packed, prelude::Pack, H160, H256};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

pub const SECP256K1: &str = "secp256k1_blake160";
pub const ACP: &str = "anyone_can_pay";
pub const CHEQUE: &str = "cheque";

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    PayByFrom = 0,
    LendByFrom,
    PayByTo,
}

impl Action {
    fn to_scripts(&self) -> Vec<ScriptType> {
        match self {
            Action::PayByFrom => vec![ScriptType::Secp256k1],
            Action::LendByFrom => vec![ScriptType::Cheque],
            Action::PayByTo => vec![ScriptType::AnyoneCanPay],
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Owned = 0,
    Claimable,
}

impl Source {
    fn to_scripts(&self) -> Vec<ScriptType> {
        match self {
            Source::Owned => vec![ScriptType::Secp256k1, ScriptType::MyACP],
            Source::Claimable => vec![ScriptType::RedeemCheque],
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    RedeemCheque,
    Cheque,
    MyACP,
    AnyoneCanPay,
}

impl ScriptType {
    pub(crate) fn is_acp(&self) -> bool {
        self == &ScriptType::AnyoneCanPay
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            ScriptType::Secp256k1 => SECP256K1,
            ScriptType::Cheque | ScriptType::RedeemCheque => CHEQUE,
            ScriptType::MyACP | ScriptType::AnyoneCanPay => ACP,
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
    pub(crate) fn to_inner(&self) -> InnerAccount {
        InnerAccount {
            idents: vec![self.ident.clone()],
            scripts: self.action.to_scripts(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferPayload {
    pub udt_hash: Option<H256>,
    pub from: FromAccount,
    pub items: Vec<TransferItem>,
    pub change: Option<String>,
    pub fee: u64,
}

impl TransferPayload {
    pub(crate) fn to_inner_items(&self) -> Vec<InnerTransferItem> {
        self.items.iter().map(|item| item.to_inner()).collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferItem {
    pub to: ToAccount,
    pub amount: u128,
}

impl TransferItem {
    pub(crate) fn to_inner(&self) -> InnerTransferItem {
        InnerTransferItem {
            to: self.to.to_inner(),
            amount: self.amount,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferCompletionResponse {
    pub tx_view: TransactionView,
    pub sig_entry: Vec<SignatureEntry>,
}

impl TransferCompletionResponse {
    pub fn new(tx_view: TransactionView, sig_entry: Vec<SignatureEntry>) -> Self {
        TransferCompletionResponse { tx_view, sig_entry }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SignatureEntry {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub type_: WitnessType,
    pub index: usize,
    pub pub_key: H160,
    pub message: Bytes,
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
pub struct DetailedCell {
    pub cell: packed::CellOutput,
    pub data: packed::Bytes,
}

impl DetailedCell {
    pub fn new(cell: packed::CellOutput, data: Bytes) -> Self {
        DetailedCell {
            cell,
            data: data.pack(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct DetailedAmount {
    pub udt_amount: u128,
    pub ckb_all: u64,
    pub ckb_by_owned: u64,
    pub ckb_lend: u64,
    pub ckb_by_acp: HashMap<String, u64>,
}

impl DetailedAmount {
    pub fn new() -> Self {
        DetailedAmount::default()
    }

    pub fn add_udt_amount(&mut self, amount: u128) {
        self.udt_amount += amount;
    }

    pub fn add_ckb_by_owned(&mut self, amount: u64) {
        self.ckb_by_owned += amount;
        self.ckb_all += amount;
    }

    pub fn add_ckb_lend(&mut self, amount: u64) {
        self.ckb_lend += amount;
        self.ckb_all += amount;
    }

    pub fn add_ckb_by_acp(&mut self, addr: String, amount: u64) {
        *self.ckb_by_acp.entry(addr).or_insert(0) += amount;
    }

    pub fn add_ckb_all(&mut self, amount: u64) {
        self.ckb_all += amount;
    }

    #[cfg(test)]
    fn ckb_all(&self) -> u64 {
        self.ckb_all
    }
}

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
    detailed_cells: Vec<DetailedCell>,
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

    use crate::extensions::anyone_can_pay;
    use rand::random;

    #[test]
    fn test_detailed_amount() {
        let mut amount = DetailedAmount::default();

        let udt = random::<u128>();
        let ckb_owe = random::<u32>();
        let ckb_lend = random::<u32>();
        let ckb_acp = random::<u32>();
        let ckb_all = (ckb_owe as u64) + (ckb_lend as u64);

        amount.add_udt_amount(udt);
        amount.add_ckb_by_owned(ckb_owe as u64);
        amount.add_ckb_lend(ckb_lend as u64);
        amount.add_ckb_by_acp(String::from("addr000"), ckb_acp as u64);

        assert_eq!(amount.ckb_all(), ckb_all);
    }

    #[test]
    fn test_constant_eq() {
        assert_eq!(ACP, anyone_can_pay::ACP);
    }
}
