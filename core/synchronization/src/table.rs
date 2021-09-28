use core_storage::relational::{table::BsonBytes, to_bson_bytes};
use core_storage::single_sql_return;
use db_xsql::rbatis::crud_table;

use ckb_types::{packed, prelude::*};
use serde::{Deserialize, Serialize};

single_sql_return!(ScriptHash, script_hash, BsonBytes);

#[crud_table(table_name: "mercury_sync_status")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncStatus {
    pub block_number: u64,
}

impl SyncStatus {
    pub fn new(block_number: u64) -> SyncStatus {
        SyncStatus { block_number }
    }
}

#[crud_table(
    table_name: "mercury_consume_info" | formats_pg: "
    tx_hash:{}::bytea,
    consumed_block_hash:{}::bytea,
    consumed_tx_hash:{}::bytea,
    since:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConsumeInfoTable {
    pub tx_hash: BsonBytes,
    pub output_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: BsonBytes,
    pub consumed_tx_hash: BsonBytes,
    pub consumed_tx_index: u32,
    pub input_index: u32,
    pub since: BsonBytes,
}

impl ConsumeInfoTable {
    pub fn new(
        out_point: packed::OutPoint,
        consumed_block_number: u64,
        consumed_block_hash: BsonBytes,
        consumed_tx_hash: BsonBytes,
        consumed_tx_index: u32,
        input_index: u32,
        since: u64,
    ) -> Self {
        let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
        let output_index: u32 = out_point.index().unpack();

        ConsumeInfoTable {
            tx_hash,
            output_index,
            consumed_block_number,
            consumed_block_hash,
            consumed_tx_hash,
            consumed_tx_index,
            input_index,
            since: to_bson_bytes(&since.to_be_bytes()),
        }
    }
}

#[crud_table(table_name: "mercury_in_update")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InUpdate {
    pub is_in: bool,
}
