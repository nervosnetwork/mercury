use crate::table::BsonBytes;

use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::sql;

#[sql(
    tx,
    "UPDATE cell SET
    consumed_block_number = $1, 
    consumed_block_hash = $2::bytea, 
    consumed_tx_hash = $3::bytea, 
    consumed_tx_index = $4, 
    input_index = $5, 
    since = $6 
    WHERE tx_hash = $7::bytea AND output_index = $8"
)]
pub async fn update_consume_cell(
    tx: &mut RBatisTxExecutor<'_>,
    consumed_block_number: u64,
    consumed_block_hash: BsonBytes,
    consumed_tx_hash: BsonBytes,
    consumed_tx_index: u16,
    input_index: u16,
    since: u64,
    tx_hash: BsonBytes,
    output_index: u32,
) -> () {
}

#[sql(conn, "SELECT id FROM script WHERE script_hash = $1::bytea limit 1")]
pub async fn has_script_hash(
    conn: &mut RBatisConnExecutor<'_>,
    script_hash: BsonBytes,
) -> Option<i64> {
}

#[sql(
    conn,
    "SELECT id FROM live_cell WHERE tx_hash = $1::bytea AND output_index = $2"
)]
pub async fn is_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
    index: u16,
) -> Option<i64> {
}

#[sql(
    conn,
    "DELETE FROM live_cell WHERE tx_hash = $1::bytea AND output_index = $2"
)]
pub async fn remove_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
    index: u16,
) -> () {
}
