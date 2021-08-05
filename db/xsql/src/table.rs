use rbatis::crud_table;
use serde::{Deserialize, Serialize};

#[crud_table(table_name: "block")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: String,
    pub block_number: u64,
    pub version: u32,
    pub compact_target: u64,
    pub timestamp: u64,
    pub epoch: u64,
    pub parent_hash: String,
    pub transactions_root: String,
    pub proposals_hash: String,
    pub uncles_hash: String,
    pub dao: String,
    pub nonce: u64,
    pub proposals: String,
}

#[crud_table(table_name: "transaction")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TransactionTable {
    pub tx_hash: String,
    pub tx_index: u32,
    pub input_count: u32,
    pub output_count: u32,
    pub block_number: u64,
    pub timestamp: u64,
    pub version: u32,
    pub cell_deps: Vec<u8>,
    pub header_deps: Vec<u8>,
    pub witness: Vec<u8>,
}

#[crud_table(table_name: "cell")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CellTable {
    pub tx_hash: String,
    pub output_index: u32,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_index: u32,
    pub epoch_number: u64,
    pub capacity: u64,
    pub lock_hash: String,
    pub lock_code_hash: String,
    pub lock_args: String,
    pub lock_script_type: u8,
    pub type_hash: Option<String>,
    pub type_code_hash: Option<String>,
    pub type_args: Option<String>,
    pub type_script_type: u8,
    pub extra_field: u32,
    pub data: Option<String>,
    pub is_data_complete: bool,
    pub consumed_block_number: Option<u64>,
    pub consumed_block_hash: Option<String>,
    pub consumed_tx_hash: Option<String>,
    pub consumed_tx_index: Option<u64>,
    pub input_index: Option<u32>,
    pub since: u64,
}

#[crud_table(table_name: "script")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ScriptTable {
    pub script_hash: String,
    pub script_hash_160: String,
    pub code_hash: String,
    pub script_args: String,
    pub script_type: u8,
    pub script_args_len: u32,
}

#[crud_table(table_name: "tip")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TipTable {
    pub tip_hash: String,
    pub tip_number: u64,
}

#[crud_table(table_name: "live_cell")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct LiveCellTable {
    pub tx_hash: String,
    pub output_index: u32,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_index: u32,
    pub epoch_number: u64,
    pub capacity: u64,
    pub lock_hash: String,
    pub lock_code_hash: String,
    pub lock_args: String,
    pub lock_script_type: u8,
    pub type_hash: Option<String>,
    pub type_code_hash: Option<String>,
    pub type_args: Option<String>,
    pub type_script_type: u8,
    pub extra_field: u32,
    pub data: Option<String>,
    pub is_data_complete: bool,
}

#[crud_table(table_name: "big_data")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BigDataTable {
    pub tx_hash: String,
    pub output_index: u32,
    pub data: Vec<u8>,
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
