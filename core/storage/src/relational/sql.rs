#![allow(
    clippy::assign_op_pattern,
    clippy::manual_range_contains,
    clippy::modulo_one
)]

use crate::relational::table::TxHash;

use db_xsql::rbatis::executor::RBatisTxExecutor;
use db_xsql::rbatis::{sql, Bytes as RbBytes};

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
    empty_bytes: &RbBytes,
    consumed_tx_hash: &RbBytes,
) -> () {
}

#[sql(tx, "SELECT tx_hash FROM mercury_transaction WHERE block_hash = $1")]
pub async fn get_tx_hashes_by_block_hash(
    tx: &mut RBatisTxExecutor<'_>,
    block_hash: &RbBytes,
) -> Vec<TxHash> {
}
