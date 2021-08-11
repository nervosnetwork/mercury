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
    consumed_block_hash: Vec<u8>,
    consumed_tx_hash: Vec<u8>,
    consumed_tx_index: u32,
    input_index: u32,
    since: u64,
    tx_hash: Vec<u8>,
    output_index: u32,
) -> () {
}

#[sql(conn, "SELECT id FROM script where script_hash = $1::bytea limit 1")]
pub async fn has_script_hash(conn: &mut RBatisConnExecutor<'_>, script_hash: &[u8]) -> Option<i64> {
}
