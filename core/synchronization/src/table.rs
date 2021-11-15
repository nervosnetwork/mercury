use core_storage::relational::to_rb_bytes;
use core_storage::single_sql_return;
use db_xsql::rbatis::{crud_table, Bytes as RbBytes};

use ckb_types::{packed, prelude::*};
use serde::{Deserialize, Serialize};

single_sql_return!(ScriptHash, script_hash, RbBytes);
single_sql_return!(SyncNumber, block_number, u64);

#[crud_table(table_name: "mercury_consume_info")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConsumeInfoTable {
    pub tx_hash: RbBytes,
    pub output_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: RbBytes,
    pub consumed_tx_hash: RbBytes,
    pub consumed_tx_index: u32,
    pub input_index: u32,
    pub since: RbBytes,
}

impl ConsumeInfoTable {
    pub fn new(
        out_point: packed::OutPoint,
        consumed_block_number: u64,
        consumed_block_hash: RbBytes,
        consumed_tx_hash: RbBytes,
        consumed_tx_index: u32,
        input_index: u32,
        since: u64,
    ) -> Self {
        let tx_hash = to_rb_bytes(&out_point.tx_hash().raw_data());
        let output_index: u32 = out_point.index().unpack();

        ConsumeInfoTable {
            tx_hash,
            output_index,
            consumed_block_number,
            consumed_block_hash,
            consumed_tx_hash,
            consumed_tx_index,
            input_index,
            since: to_rb_bytes(&since.to_be_bytes()),
        }
    }
}

#[crud_table(table_name: "mercury_in_update")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InUpdate {
    pub is_in: bool,
}
