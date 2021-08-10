use crate::str;

use ckb_types::{packed, prelude::*};
use rbatis::{crud_table, plugin::snowflake::SNOWFLAKE};
use serde::{Deserialize, Serialize};

const BLAKE_160_STR_LEN: usize = 20 * 2;

#[crud_table(table_name: "block")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: String,
    pub block_number: u64,
    pub version: u32,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch: u64,
    pub parent_hash: String,
    pub transactions_root: String,
    pub proposals_hash: String,
    pub uncles_hash: String,
    pub dao: String,
    pub nonce: String,
    pub proposals: String,
}

#[crud_table(table_name: "transaction")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: String,
    pub tx_index: u32,
    pub input_count: u32,
    pub output_count: u32,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_timestamp: u64,
    pub version: u32,
    pub cell_deps: String,
    pub header_deps: String,
    pub witnesses: String,
}

#[crud_table(
    table_name: "cell" | formats_pg: "type_hash:{}::char, type_code_hash:{}::char,type_args:{}::char,type_script_type:{}::int,data:{}::char,consumed_block_number:{}::int,consumed_block_hash:{}::char,consumed_tx_hash:{}::char,consumed_tx_index:{}::int,input_index:{}::int,since:{}::int"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: String,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: String,
    pub epoch_number: u64,
    pub capacity: u64,
    pub lock_hash: String,
    pub lock_code_hash: String,
    pub lock_args: String,
    pub lock_script_type: u8,
    pub type_hash: Option<String>,
    pub type_code_hash: Option<String>,
    pub type_args: Option<String>,
    pub type_script_type: Option<u8>,
    pub data: Option<String>,
    pub is_data_complete: bool,
    pub consumed_block_number: Option<u64>,
    pub consumed_block_hash: Option<String>,
    pub consumed_tx_hash: Option<String>,
    pub consumed_tx_index: Option<u32>,
    pub input_index: Option<u32>,
    pub since: Option<u64>,
}

impl CellTable {
    pub fn set_type_script_info(&mut self, script: &packed::Script) {
        self.type_hash = Some(str!(script.calc_script_hash()));
        self.type_code_hash = Some(str!(script.code_hash()));
        self.type_args = Some(str!(script.args()));
        self.type_script_type = Some(script.hash_type().into());
    }

    pub fn to_lock_script_table(&self) -> ScriptTable {
        ScriptTable {
            script_hash: self.lock_hash.clone(),
            script_args: self.lock_args.clone(),
            script_args_len: (self.lock_args.len() / 2) as u32,
            script_code_hash: self.lock_code_hash.clone(),
            script_type: self.lock_script_type,
            id: SNOWFLAKE.generate(),
            script_hash_160: self
                .lock_hash
                .as_str()
                .split_at(BLAKE_160_STR_LEN)
                .0
                .to_string(),
        }
    }

    pub fn to_type_script_table(&self) -> ScriptTable {
        let type_hash = self.type_hash.clone().unwrap();
        let type_script_args = self.type_args.clone().unwrap();

        ScriptTable {
            script_hash: type_hash.clone(),
            script_hash_160: type_hash.as_str().split_at(BLAKE_160_STR_LEN).0.to_string(),
            script_args_len: (type_script_args.len() / 2) as u32,
            script_args: type_script_args,
            script_code_hash: self.type_code_hash.clone().unwrap(),
            script_type: self.type_script_type.unwrap(),
            id: SNOWFLAKE.generate(),
        }
    }

    pub fn into_live_cell_table(self) -> LiveCellTable {
        self.into()
    }
}

#[crud_table(table_name: "script")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ScriptTable {
    pub id: i64,
    pub script_hash: String,
    pub script_hash_160: String,
    pub script_code_hash: String,
    pub script_args: String,
    pub script_type: u8,
    pub script_args_len: u32,
}

#[crud_table(
    table_name: "live_cell" | formats_pg: "type_hash:{}::char, type_code_hash:{}::char,type_args:{}::char,type_script_type:{}::int,data:{}::char"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: String,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: String,
    pub epoch_number: u64,
    pub capacity: u64,
    pub lock_hash: String,
    pub lock_code_hash: String,
    pub lock_args: String,
    pub lock_script_type: u8,
    pub type_hash: Option<String>,
    pub type_code_hash: Option<String>,
    pub type_args: Option<String>,
    pub type_script_type: Option<u8>,
    pub data: Option<String>,
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

#[crud_table(table_name: "big_data")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BigDataTable {
    pub tx_hash: String,
    pub output_index: u32,
    pub data: String,
}

#[crud_table(table_name: "uncle_relationship")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct UncleRelationshipTable {
    pub block_hash: String,
    pub uncles_hash: String,
}

#[crud_table(table_name: "canonical_chain")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CanonicalChainTable {
    pub block_num: u64,
    pub block_hash: String,
}
