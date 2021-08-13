use ckb_types::core::{BlockView, TransactionView};
use ckb_types::{packed, prelude::*};
use rbatis::crud_table;
use serde::{Deserialize, Serialize};

use std::cmp::{Ord, Ordering, PartialOrd};

const BLAKE_160_HSAH_LEN: usize = 20;

#[crud_table(
    table_name: "block" | formats_pg: "
    block_hash:{}::bytea,
    parent_hash:{}::bytea,
    transactions_root:{}::bytea,
    proposals_hash:{}::bytea,
    uncles_hash:{}::bytea,
    dao:{}::bytea,
    nonce:{}::bytea,
    proposals:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: Vec<u8>,
    pub block_number: u64,
    pub version: u16,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch_number: u64,
    pub epoch_length: u16,
    pub epoch_block_index: u16,
    pub parent_hash: Vec<u8>,
    pub transactions_root: Vec<u8>,
    pub proposals_hash: Vec<u8>,
    pub uncles_hash: Vec<u8>,
    pub dao: Vec<u8>,
    pub nonce: Vec<u8>,
    pub proposals: Vec<u8>,
}

impl From<&BlockView> for BlockTable {
    fn from(block: &BlockView) -> Self {
        let epoch = block.epoch();
        BlockTable {
            block_hash: block.hash().raw_data().to_vec(),
            block_number: block.number(),
            version: block.version() as u16,
            compact_target: block.compact_target(),
            block_timestamp: block.timestamp(),
            epoch_number: epoch.number(),
            epoch_block_index: epoch.index() as u16,
            epoch_length: epoch.length() as u16,
            parent_hash: block.parent_hash().raw_data().to_vec(),
            transactions_root: block.transactions_root().raw_data().to_vec(),
            proposals_hash: block.proposals_hash().raw_data().to_vec(),
            uncles_hash: block.uncles_hash().raw_data().to_vec(),
            dao: block.dao().raw_data().to_vec(),
            nonce: block.nonce().to_be_bytes().to_vec(),
            proposals: block.data().proposals().as_bytes().to_vec(),
        }
    }
}

#[crud_table(
    table_name: "transaction" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    cell_deps:{}::bytea,
    header_deps:{}::bytea,
    witnesses:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub tx_index: u16,
    pub input_count: u16,
    pub output_count: u16,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub tx_timestamp: u64,
    pub version: u16,
    pub cell_deps: Vec<u8>,
    pub header_deps: Vec<u8>,
    pub witnesses: Vec<u8>,
}

impl TransactionTable {
    pub fn from_view(
        view: &TransactionView,
        id: i64,
        index: u16,
        block_hash: Vec<u8>,
        block_timestamp: u64,
        block_number: u64,
    ) -> Self {
        TransactionTable {
            id,
            block_hash,
            tx_hash: view.hash().raw_data().to_vec(),
            tx_index: index,
            tx_timestamp: block_timestamp,
            input_count: view.inputs().len() as u16,
            output_count: view.outputs().len() as u16,
            cell_deps: view.cell_deps().as_bytes().to_vec(),
            header_deps: view.header_deps().as_bytes().to_vec(),
            witnesses: view.witnesses().as_bytes().to_vec(),
            version: view.version() as u16,
            block_number,
        }
    }
}

#[crud_table(
    table_name: "cell" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    epoch_number:{}::bytea,
    lock_hash:{}::bytea,
    lock_code_hash:{}::bytea,
    lock_args:{}::bytea,
    type_hash:{}::bytea,
    type_code_hash:{}::bytea,
    type_args:{}::bytea,
    type_script_type:{}::smallint,
    data:{}::bytea,
    consumed_block_hash:{}::bytea,
    consumed_tx_hash:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub output_index: u16,
    pub tx_index: u16,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub epoch_number: Vec<u8>,
    pub capacity: u64,
    pub lock_hash: Vec<u8>,
    pub lock_code_hash: Vec<u8>,
    pub lock_args: Vec<u8>,
    pub lock_script_type: u8,
    pub type_hash: Vec<u8>,
    pub type_code_hash: Vec<u8>,
    pub type_args: Vec<u8>,
    pub type_script_type: u8,
    pub data: Vec<u8>,
    pub is_data_complete: bool,
    pub consumed_block_number: Option<u64>,
    pub consumed_block_hash: Vec<u8>,
    pub consumed_tx_hash: Vec<u8>,
    pub consumed_tx_index: Option<u16>,
    pub input_index: Option<u16>,
    pub since: Option<u64>,
}

impl CellTable {
    pub fn from_cell(
        cell: &packed::CellOutput,
        id: i64,
        tx_hash: Vec<u8>,
        output_index: u16,
        tx_index: u16,
        block_number: u64,
        block_hash: Vec<u8>,
        epoch_number: u64,
    ) -> Self {
        let mut ret = CellTable {
            id,
            tx_hash,
            output_index,
            tx_index,
            block_number,
            block_hash,
            epoch_number: epoch_number.to_be_bytes().to_vec(),
            capacity: cell.capacity().unpack(),
            lock_hash: cell.lock().calc_script_hash().raw_data().to_vec(),
            lock_code_hash: cell.lock().code_hash().raw_data().to_vec(),
            lock_args: cell.lock().args().raw_data().to_vec(),
            lock_script_type: cell.lock().hash_type().into(),
            ..Default::default()
        };

        if let Some(script) = cell.type_().to_opt() {
            ret.set_type_script_info(&script);
        }

        ret
    }

    pub fn has_type_script(&self) -> bool {
        !self.type_hash.is_empty()
    }

    pub fn set_type_script_info(&mut self, script: &packed::Script) {
        self.type_hash = script.calc_script_hash().raw_data().to_vec();
        self.type_code_hash = script.code_hash().raw_data().to_vec();
        self.type_args = script.args().raw_data().to_vec();
        self.type_script_type = script.hash_type().into();
    }

    pub fn to_lock_script_table(&self, id: i64) -> ScriptTable {
        ScriptTable {
            script_hash: self.lock_hash.clone(),
            script_args: self.lock_args.clone(),
            script_args_len: self.lock_args.len() as u16,
            script_code_hash: self.lock_code_hash.clone(),
            script_type: self.lock_script_type,
            script_hash_160: self.lock_hash.split_at(BLAKE_160_HSAH_LEN).0.to_owned(),
            id,
        }
    }

    pub fn to_type_script_table(&self, id: i64) -> ScriptTable {
        let type_hash = self.type_hash.clone();
        let type_script_args = self.type_args.clone();

        ScriptTable {
            script_hash: type_hash.clone(),
            script_hash_160: type_hash.split_at(BLAKE_160_HSAH_LEN).0.to_owned(),
            script_args_len: type_script_args.len() as u16,
            script_args: type_script_args,
            script_code_hash: self.type_code_hash.clone(),
            script_type: self.type_script_type,
            id,
        }
    }

    pub fn into_live_cell_table(self) -> LiveCellTable {
        self.into()
    }
}

#[crud_table(
    table_name: "script" | formats_pg:"
    script_hash:{}::bytea,
    script_hash_160:{}::bytea,
    script_code_hash:{}::bytea,
    script_args:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ScriptTable {
    pub id: i64,
    pub script_hash: Vec<u8>,
    pub script_hash_160: Vec<u8>,
    pub script_code_hash: Vec<u8>,
    pub script_args: Vec<u8>,
    pub script_type: u8,
    pub script_args_len: u16,
}

#[crud_table(
    table_name: "live_cell" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    epoch_number:{}::bytea,
    lock_hash:{}::bytea,
    lock_code_hash:{}::bytea,
    lock_args:{}::bytea,
    type_hash:{}::bytea,
    type_code_hash:{}::bytea,
    type_args:{}::bytea,
    type_script_type:{}::int,
    data:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub output_index: u16,
    pub tx_index: u16,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub epoch_number: Vec<u8>,
    pub capacity: u64,
    pub lock_hash: Vec<u8>,
    pub lock_code_hash: Vec<u8>,
    pub lock_args: Vec<u8>,
    pub lock_script_type: u8,
    pub type_hash: Vec<u8>,
    pub type_code_hash: Vec<u8>,
    pub type_args: Vec<u8>,
    pub type_script_type: u8,
    pub data: Vec<u8>,
    pub is_data_complete: bool,
}

impl From<CellTable> for LiveCellTable {
    fn from(s: CellTable) -> Self {
        LiveCellTable {
            id: s.id,
            tx_hash: s.tx_hash,
            output_index: s.output_index,
            block_number: s.block_number,
            block_hash: s.block_hash,
            tx_index: s.tx_index,
            epoch_number: s.epoch_number,
            capacity: s.capacity,
            lock_hash: s.lock_hash,
            lock_code_hash: s.lock_code_hash,
            lock_args: s.lock_args,
            lock_script_type: s.lock_script_type,
            type_hash: s.type_hash,
            type_code_hash: s.type_code_hash,
            type_args: s.type_args,
            type_script_type: s.type_script_type,
            data: s.data,
            is_data_complete: s.is_data_complete,
        }
    }
}

#[crud_table(
    table_name: "big_data"| formats_pg: "
    tx_hash:{}::bytea,
    data:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BigDataTable {
    pub tx_hash: Vec<u8>,
    pub output_index: u16,
    pub data: Vec<u8>,
}

#[crud_table(table_name: "uncle_relationship" | formats_pg: "
    block_hash:{}::bytea,
    uncles_hash:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct UncleRelationshipTable {
    pub block_hash: Vec<u8>,
    pub uncles_hash: Vec<u8>,
}

#[crud_table(table_name: "canonical_chain" | formats_pg: "block_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct CanonicalChainTable {
    pub block_num: u64,
    pub block_hash: Vec<u8>,
}

impl PartialOrd for CanonicalChainTable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.block_num.partial_cmp(&other.block_num)
    }
}

impl Ord for CanonicalChainTable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_num.cmp(&other.block_num)
    }
}
