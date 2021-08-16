use crate::{empty_bson_bytes, to_bson_bytes};

use bson::Binary;
use ckb_types::core::{BlockView, TransactionView};
use ckb_types::{packed, prelude::*};
use rbatis::crud_table;
use serde::{Deserialize, Serialize};

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::hash::{Hash, Hasher};

pub type BsonBytes = Binary;

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: BsonBytes,
    pub block_number: u64,
    pub version: u16,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch_number: u64,
    pub epoch_length: u16,
    pub epoch_block_index: u16,
    pub parent_hash: BsonBytes,
    pub transactions_root: BsonBytes,
    pub proposals_hash: BsonBytes,
    pub uncles_hash: BsonBytes,
    pub dao: BsonBytes,
    pub nonce: BsonBytes,
    pub proposals: BsonBytes,
}

impl From<&BlockView> for BlockTable {
    fn from(block: &BlockView) -> Self {
        let epoch = block.epoch();
        BlockTable {
            block_hash: to_bson_bytes(&block.hash().raw_data()),
            block_number: block.number(),
            version: block.version() as u16,
            compact_target: block.compact_target(),
            block_timestamp: block.timestamp(),
            epoch_number: epoch.number(),
            epoch_block_index: epoch.index() as u16,
            epoch_length: epoch.length() as u16,
            parent_hash: to_bson_bytes(&block.parent_hash().raw_data()),
            transactions_root: to_bson_bytes(&block.transactions_root().raw_data()),
            proposals_hash: to_bson_bytes(&block.proposals_hash().raw_data()),
            uncles_hash: to_bson_bytes(&block.uncles_hash().raw_data()),
            dao: to_bson_bytes(&block.dao().raw_data()),
            nonce: to_bson_bytes(&block.nonce().to_be_bytes()),
            proposals: to_bson_bytes(&block.data().proposals().as_bytes()),
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub tx_index: u16,
    pub input_count: u16,
    pub output_count: u16,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub tx_timestamp: u64,
    pub version: u16,
    pub cell_deps: BsonBytes,
    pub header_deps: BsonBytes,
    pub witnesses: BsonBytes,
}

impl TransactionTable {
    pub fn from_view(
        view: &TransactionView,
        id: i64,
        index: u16,
        block_hash: BsonBytes,
        block_timestamp: u64,
        block_number: u64,
    ) -> Self {
        TransactionTable {
            id,
            block_hash,
            tx_hash: to_bson_bytes(&view.hash().raw_data()),
            tx_index: index,
            tx_timestamp: block_timestamp,
            input_count: view.inputs().len() as u16,
            output_count: view.outputs().len() as u16,
            cell_deps: to_bson_bytes(&view.cell_deps().as_bytes()),
            header_deps: to_bson_bytes(&view.header_deps().as_bytes()),
            witnesses: to_bson_bytes(&view.witnesses().as_bytes()),
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub output_index: u16,
    pub tx_index: u16,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub epoch_number: BsonBytes,
    pub capacity: u64,
    pub lock_hash: BsonBytes,
    pub lock_code_hash: BsonBytes,
    pub lock_args: BsonBytes,
    pub lock_script_type: u8,
    pub type_hash: BsonBytes,
    pub type_code_hash: BsonBytes,
    pub type_args: BsonBytes,
    pub type_script_type: u8,
    pub data: BsonBytes,
    pub is_data_complete: bool,
    pub consumed_block_number: Option<u64>,
    pub consumed_block_hash: BsonBytes,
    pub consumed_tx_hash: BsonBytes,
    pub consumed_tx_index: Option<u16>,
    pub input_index: Option<u16>,
    pub since: Option<u64>,
}

impl CellTable {
    pub fn from_cell(
        cell: &packed::CellOutput,
        id: i64,
        tx_hash: BsonBytes,
        output_index: u16,
        tx_index: u16,
        block_number: u64,
        block_hash: BsonBytes,
        epoch_number: u64,
    ) -> Self {
        let mut ret = CellTable {
            id,
            tx_hash,
            output_index,
            tx_index,
            block_number,
            block_hash,
            epoch_number: to_bson_bytes(&epoch_number.to_be_bytes()),
            capacity: cell.capacity().unpack(),
            lock_hash: to_bson_bytes(&cell.lock().calc_script_hash().raw_data()),
            lock_code_hash: to_bson_bytes(&cell.lock().code_hash().raw_data()),
            lock_args: to_bson_bytes(&cell.lock().args().raw_data()),
            lock_script_type: cell.lock().hash_type().into(),
            type_hash: empty_bson_bytes(),
            type_code_hash: empty_bson_bytes(),
            type_args: empty_bson_bytes(),
            type_script_type: 0u8,
            data: empty_bson_bytes(),
            is_data_complete: true,
            consumed_block_number: None,
            consumed_block_hash: empty_bson_bytes(),
            consumed_tx_hash: empty_bson_bytes(),
            consumed_tx_index: None,
            input_index: None,
            since: None,
        };

        if let Some(script) = cell.type_().to_opt() {
            ret.set_type_script_info(&script);
        }

        ret
    }

    pub fn has_type_script(&self) -> bool {
        !self.type_hash.bytes.is_empty()
    }

    pub fn set_type_script_info(&mut self, script: &packed::Script) {
        self.type_hash = to_bson_bytes(&script.calc_script_hash().raw_data());
        self.type_code_hash = to_bson_bytes(&script.code_hash().raw_data());
        self.type_args = to_bson_bytes(&script.args().raw_data());
        self.type_script_type = script.hash_type().into();
    }

    pub fn to_lock_script_table(&self, id: i64) -> ScriptTable {
        ScriptTable {
            script_hash: self.lock_hash.clone(),
            script_args: self.lock_args.clone(),
            script_args_len: self.lock_args.bytes.len() as u16,
            script_code_hash: self.lock_code_hash.clone(),
            script_type: self.lock_script_type,
            script_hash_160: to_bson_bytes(self.lock_hash.bytes.split_at(BLAKE_160_HSAH_LEN).0),
            id,
        }
    }

    pub fn to_type_script_table(&self, id: i64) -> ScriptTable {
        let type_hash = self.type_hash.clone();
        let type_script_args = self.type_args.clone();

        ScriptTable {
            script_hash: type_hash.clone(),
            script_hash_160: to_bson_bytes(&type_hash.bytes.split_at(BLAKE_160_HSAH_LEN).0),
            script_args_len: type_script_args.bytes.len() as u16,
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptTable {
    pub id: i64,
    pub script_hash: BsonBytes,
    pub script_hash_160: BsonBytes,
    pub script_code_hash: BsonBytes,
    pub script_args: BsonBytes,
    pub script_type: u8,
    pub script_args_len: u16,
}

impl Hash for ScriptTable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.script_hash.bytes.hash(state);
        self.script_code_hash.bytes.hash(state);
        self.script_type.hash(state);
        self.script_args.bytes.hash(state);
    }
}

impl PartialEq for ScriptTable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.script_hash == other.script_hash
            && self.script_code_hash == other.script_code_hash
            && self.script_type == other.script_type
            && self.script_args == other.script_args
    }
}

impl Eq for ScriptTable {}

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub output_index: u16,
    pub tx_index: u16,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub epoch_number: BsonBytes,
    pub capacity: u64,
    pub lock_hash: BsonBytes,
    pub lock_code_hash: BsonBytes,
    pub lock_args: BsonBytes,
    pub lock_script_type: u8,
    pub type_hash: BsonBytes,
    pub type_code_hash: BsonBytes,
    pub type_args: BsonBytes,
    pub type_script_type: u8,
    pub data: BsonBytes,
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BigDataTable {
    pub tx_hash: BsonBytes,
    pub output_index: u16,
    pub data: BsonBytes,
}

#[crud_table(table_name: "uncle_relationship" | formats_pg: "
    block_hash:{}::bytea,
    uncles_hash:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UncleRelationshipTable {
    pub block_hash: BsonBytes,
    pub uncles_hash: BsonBytes,
}

#[crud_table(table_name: "canonical_chain" | formats_pg: "block_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalChainTable {
    pub block_number: u64,
    pub block_hash: BsonBytes,
}

impl PartialEq for CanonicalChainTable {
    fn eq(&self, other: &Self) -> bool {
        self.block_number == other.block_number && self.block_hash.bytes == other.block_hash.bytes
    }
}

impl Eq for CanonicalChainTable {}

impl PartialOrd for CanonicalChainTable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.block_number.partial_cmp(&other.block_number)
    }
}

impl Ord for CanonicalChainTable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_number.cmp(&other.block_number)
    }
}
