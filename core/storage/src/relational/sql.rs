use crate::relational::table::{BsonBytes, MercuryId, ScriptTable, TxHash};

use db_xsql::rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use db_xsql::rbatis::sql;

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
    consumed_block_hash = $1::bytea,
    consumed_block_number = NULL,
    consumed_tx_hash = $1::bytea,
    consumed_tx_index = NULL,
    input_index = NULL,
    since = $1::bytea WHERE consumed_tx_hash = $2::bytea"
)]
pub async fn rollback_consume_cell(
    tx: &mut RBatisTxExecutor<'_>,
    empty_bytes: BsonBytes,
    consumed_tx_hash: BsonBytes,
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
#[sql(conn, "SELECT COUNT(1) FROM mercury_consume_info")]
pub async fn fetch_cunsumed_cell_count(conn: &mut RBatisConnExecutor<'_>) -> u64 {}
