use crate::table::{BsonBytes, ScriptTable};

use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::sql;

#[sql(
    tx,
    "UPDATE mercury_cell SET
    consumed_block_number = $1, 
    consumed_block_hash = $2::bytea, 
    consumed_tx_hash = $3::bytea, 
    consumed_tx_index = $4, 
    input_index = $5, 
    since = $6::bytea
    WHERE tx_hash = $7::bytea AND output_index = $8"
)]
pub async fn update_consume_cell(
    tx: &mut RBatisTxExecutor<'_>,
    consumed_block_number: u64,
    consumed_block_hash: BsonBytes,
    consumed_tx_hash: BsonBytes,
    consumed_tx_index: u32,
    input_index: u32,
    since: BsonBytes,
    tx_hash: BsonBytes,
    output_index: u32,
) -> () {
}

#[sql(
    tx,
    "UPDATE mercury_cell SET
    consumed_block_number = $1, 
    consumed_block_hash = $2, 
    consumed_tx_hash = $3, 
    consumed_tx_index = $4, 
    input_index = $5, 
    since = $6 
    WHERE tx_hash = $7 AND output_index = $8"
)]
pub async fn update_consume_cell_sqlite(
    tx: &mut RBatisTxExecutor<'_>,
    consumed_block_number: u64,
    consumed_block_hash: BsonBytes,
    consumed_tx_hash: BsonBytes,
    consumed_tx_index: u16,
    input_index: u16,
    since: BsonBytes,
    tx_hash: BsonBytes,
    output_index: u16,
) -> () {
}

#[sql(
    conn,
    "SELECT id FROM mercury_live_cell WHERE tx_hash = $1::bytea AND output_index = $2"
)]
pub async fn is_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
    index: u16,
) -> Option<i64> {
}

#[sql(
    conn,
    "DELETE FROM mercury_live_cell WHERE tx_hash = $1::bytea AND output_index = $2"
)]
pub async fn remove_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
    index: u16,
) -> () {
}

#[sql(
    tx,
    "SELECT tx_hash FROM mercury_transaction WHERE tx_hash = $1::bytea"
)]
pub async fn get_tx_hash_by_block_hash(
    tx: &mut RBatisTxExecutor<'_>,
    block_hash: BsonBytes,
) -> Option<Vec<BsonBytes>> {
}

#[sql(
    conn,
    "SELECT * FROM mercury_script WHERE script_code_hash = $1::bytea IN (SELECT script_code_hash FROM mercury_script WHERE substring(script_args::bytea from $3 for $4) = $2)"
)]
pub async fn query_scripts_by_partial_arg(
    conn: &mut RBatisConnExecutor<'_>,
    code_hash: BsonBytes,
    arg: BsonBytes,
    from: u32,
    to: u32,
) -> Option<Vec<ScriptTable>> {
}

#[sql(
    tx,
    "SELECT current_sync_number FROM mercury_sync_status WHERE block_range = $1"
)]
pub async fn query_current_sync_number(
    tx: &mut RBatisTxExecutor<'_>,
    block_range: u32,
) -> Option<u32> {
}

#[sql(
    tx,
    "UPDATE mercury_sync_dead_cell SET is_delete = true WHERE tx_hash = $1::bytea and output_index = $2"
)]
pub async fn update_sync_dead_cell(
    tx: &mut RBatisTxExecutor<'_>,
    tx_hash: BsonBytes,
    index: u32,
) -> () {
}
