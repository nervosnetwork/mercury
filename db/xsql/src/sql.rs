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

#[sql(tx, "SELECT tx_hash FROM transaction WHERE tx_hash = $1::bytea")]
pub async fn get_tx_hash_by_block_hash(
    tx: &mut RBatisTxExecutor<'_>,
    block_hash: BsonBytes,
) -> Option<Vec<BsonBytes>> {
}

#[cfg(test)]
#[sql(tx, "DELETE FROM block")]
pub async fn delete_block_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM transaction_")]
pub async fn delete_transaction_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM cell")]
pub async fn delete_cell_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM live_cell")]
pub async fn delete_live_cell_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM script")]
pub async fn delete_script_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM big_data")]
pub async fn delete_big_data_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM uncle_relationship")]
pub async fn delete_uncle_relationship_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM canonical_chain")]
pub async fn delete_canonical_chain_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}
