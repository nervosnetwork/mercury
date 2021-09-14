use crate::relational::table::{BsonBytes, ConsumedCell, MercuryId, ScriptTable, TxHash};

use db_xsql::rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use db_xsql::rbatis::sql;

#[sql(
    conn,
    "SELECT mercury_cell.id, mercury_cell.tx_hash, mercury_cell.output_index, mercury_cell.tx_index, 
    mercury_cell.block_number, mercury_cell.block_hash, mercury_cell.epoch_number, mercury_cell.epoch_index,
    mercury_cell.epoch_length, mercury_cell.capacity, mercury_cell.lock_hash, mercury_cell.lock_code_hash, 
    mercury_cell.lock_args, mercury_cell.lock_script_type, mercury_cell.type_hash, mercury_cell.type_code_hash, 
    mercury_cell.type_args, mercury_cell.type_script_type, mercury_cell.data, mercury_consume_info.consumed_block_number,
    mercury_consume_info.consumed_block_hash, mercury_consume_info.consumed_tx_hash, mercury_consume_info.consumed_tx_index,
    mercury_consume_info.input_index, mercury_consume_info.since
    FROM mercury_cell INNER JOIN mercury_consume_info
    ON mercury_cell.tx_hash = mercury_consume_info.tx_hash AND mercury_cell.output_index = mercury_consume_info.output_index
    WHERE mercury_consume_info.consumed_tx_hash = $1"
)]
pub async fn fetch_consume_cell_by_tx_hash(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
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
) -> Option<MercuryId> {
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

#[sql(tx, "SELECT tx_hash FROM mercury_transaction WHERE block_hash = $1")]
pub async fn get_tx_hashes_by_block_hash(
    tx: &mut RBatisTxExecutor<'_>,
    block_hash: BsonBytes,
) -> Vec<TxHash> {
}

#[sql(
    conn,
    "SELECT * FROM mercury_script 
    WHERE script_code_hash = $1::bytea AND substring(script_args::bytea ,$3::int ,$4::int) = $2::bytea"
)]
pub async fn query_scripts_by_partial_arg(
    conn: &mut RBatisConnExecutor<'_>,
    code_hash: BsonBytes,
    arg: BsonBytes,
    from: u32,
    len: u32,
) -> Vec<ScriptTable> {
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

#[cfg(test)]
#[sql(
    conn,
    "SELECT mercury_cell.id, mercury_cell.tx_hash, mercury_cell.output_index, mercury_cell.tx_index, 
    mercury_cell.block_number, mercury_cell.block_hash, mercury_cell.epoch_number, mercury_cell.epoch_index,
    mercury_cell.epoch_length, mercury_cell.capacity, mercury_cell.lock_hash, mercury_cell.lock_code_hash, 
    mercury_cell.lock_args, mercury_cell.lock_script_type, mercury_cell.type_hash, mercury_cell.type_code_hash, 
    mercury_cell.type_args, mercury_cell.type_script_type, mercury_cell.data, mercury_consume_info.consumed_block_number,
    mercury_consume_info.consumed_block_hash, mercury_consume_info.consumed_tx_hash, mercury_consume_info.consumed_tx_index,
    mercury_consume_info.input_index, mercury_consume_info.since
    FROM mercury_cell INNER JOIN mercury_consume_info
    ON mercury_cell.tx_hash = mercury_consume_info.tx_hash AND mercury_cell.output_index = mercury_consume_info.output_index"
)]
pub async fn fetch_consume_cell_by_txs_sqlite(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hashes: Vec<BsonBytes>,
) -> Vec<ConsumedCell> {
}

#[cfg(test)]
#[sql(conn, "SELECT COUNT(1) FROM mercury_consume_info")]
pub async fn fetch_cunsumed_cell_count(conn: &mut RBatisConnExecutor<'_>) -> u64 {}
