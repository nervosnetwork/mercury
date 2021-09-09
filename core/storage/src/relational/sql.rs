use crate::relational::table::{BsonBytes, ConsumedCell, ScriptTable};

use db_xsql::rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use db_xsql::rbatis::sql;

#[sql(
    conn,
    "SELECT mercury_cell.id, mercury_cell.tx_hash, mercury_cell.output_index, mercury_cell.tx_index, 
    mercury_cell.block_number, mercury_cell.block_hash, mercury_cell.epoch_number, mercury_cell.epoch_index,
    mercury_cell.epoch_length, mercury_cell.capacity, mercury_cell.lock_hash, mercury_cell.lock_code_hash, 
    mercury_cell.lock_args, mercury_cell.lock_script_type, mercury_cell.type_hash, mercury_cell.type_code_hash, 
    mercury_cell.type_args, mercury_cell.type_script_type, mercury_cell.data, mercury_consume_info.consumed_block_number,
    mercury_consume_info.consumed_block_hash, mercury_consumed_info.consumed_tx_hash, mercury_consume_info.consumed_tx_index,
    mercury_consume_info.input_index, mercury_consume_info.since
    FROM mercury_cell INNER JOIN mercury_consume_info
    On mercury_cell.tx_hash = mercury_consume_info.tx_hash AND mercury_cell.output_index = mercury_consume_info.output_index
    WHERE mercury_consume_info.consumed_tx_hash IN $1::bytea"
)]
pub async fn fetch_consume_cell_by_txs(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hashes: Vec<BsonBytes>,
) -> Vec<ConsumedCell> {
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
