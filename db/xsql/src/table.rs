use ckb_types::packed;
use rbatis::crud_table;
use serde::{Deserialize, Serialize};

const BLAKE_160_HSAH_LEN: usize = 20;

#[crud_table(table_name: "block" | formats_pg: "block_hash:{}::bytea,parent_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: Vec<u8>,
    pub block_number: u64,
    pub version: u32,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch: u64,
    pub parent_hash: Vec<u8>,
    pub transactions_root: String,
    pub proposals_hash: String,
    pub uncles_hash: String,
    pub dao: String,
    pub nonce: String,
    pub proposals: String,
}

#[crud_table(table_name: "transaction" | formats_pg: "tx_hash:{}::bytea,block_hash:{}::bytea,cell_deps:{}::bytea,header_deps:{}::bytea,witnesses:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub tx_index: u32,
    pub input_count: u32,
    pub output_count: u32,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub tx_timestamp: u64,
    pub version: u32,
    pub cell_deps: Vec<u8>,
    pub header_deps: Vec<u8>,
    pub witnesses: Vec<u8>,
}

#[crud_table(
    table_name: "cell" | formats_pg: "tx_hash:{}::bytea,block_hash:{}::bytea,lock_hash:{}::bytea,lock_code_hash:{}::bytea,lock_args:{}::bytea,type_hash:{}::bytea,type_code_hash:{}::bytea,type_args:{}::bytea,type_script_type:{}::int,data:{}::bytea,consumed_block_hash:{}::bytea,consumed_tx_hash:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub epoch_number: u64,
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
    pub consumed_tx_index: Option<u32>,
    pub input_index: Option<u32>,
    pub since: Option<u64>,
}

impl CellTable {
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
            script_args_len: self.lock_args.len() as u32,
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
            script_args_len: type_script_args.len() as u32,
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

#[crud_table(table_name: "script" | formats_pg: "script_hash:{}::bytea,script_hash_160:{}::bytea,script_code_hash:{}::bytea,script_args:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ScriptTable {
    pub id: i64,
    pub script_hash: Vec<u8>,
    pub script_hash_160: Vec<u8>,
    pub script_code_hash: Vec<u8>,
    pub script_args: Vec<u8>,
    pub script_type: u8,
    pub script_args_len: u32,
}

#[crud_table(
    table_name: "live_cell" | formats_pg: "tx_hash:{}::bytea,block_hash:{}::bytea,lock_hash:{}::bytea,lock_code_hash:{}::bytea,lock_args:{}::bytea,type_hash:{}::bytea,type_code_hash:{}::bytea,type_args:{}::bytea,type_script_type:{}::int,data:{}::bytea"
)]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub epoch_number: u64,
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

#[crud_table(table_name: "big_data"| formats_pg: "tx_hash:{}::bytea,data:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct BigDataTable {
    pub tx_hash: Vec<u8>,
    pub output_index: u32,
    pub data: Vec<u8>,
}

#[crud_table(table_name: "uncle_relationship" | formats_pg: "block_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct UncleRelationshipTable {
    pub block_hash: Vec<u8>,
    pub uncles_hash: String,
}

#[crud_table(table_name: "canonical_chain" | formats_pg: "block_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CanonicalChainTable {
    pub block_num: u64,
    pub block_hash: Vec<u8>,
}
