use crate::table::BsonBytes;

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

#[sql(
    tx,
    "SELECT id FROM mercury_script WHERE script_hash = $1::bytea limit 1"
)]
pub async fn has_script_hash(tx: &mut RBatisTxExecutor<'_>, script_hash: BsonBytes) -> Option<i64> {
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

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_block")]
pub async fn delete_block_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_transaction")]
pub async fn delete_transaction_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_cell")]
pub async fn delete_cell_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_live_cell")]
pub async fn delete_live_cell_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_script")]
pub async fn delete_script_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_uncle_relationship")]
pub async fn delete_uncle_relationship_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(tx, "DELETE FROM mercury_canonical_chain")]
pub async fn delete_canonical_chain_table_data(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_block(
    block_hash blob PRIMARY KEY,
    block_number int NOT NULL,
    version smallint NOT NULL,
    compact_target int NOT NULL,
    block_timestamp bigint NOT NULL,
    epoch_number int NOT NULL,
    epoch_block_index smallint NOT NULL,
    epoch_length smallint NOT NULL,
    parent_hash blob NOT NULL,
    transactions_root blob NOT NULL,
    proposals_hash blob NOT NULL,
    uncles_hash blob,
    dao blob NOT NULL,
    nonce blob NOT NULL,
    proposals blob
)"
)]
pub async fn create_block_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_transaction(
    id bigint PRIMARY KEY,
    tx_hash blob NOT NULL,
    tx_index smallint NOT NULL,
    input_count smallint NOT NULL,
    output_count smallint NOT NULL,
    block_number int NOT NULL,
    block_hash blob NOT NULL,
    tx_timestamp bigint NOT NULL,
    version smallint NOT NULL,
    cell_deps blob,
    header_deps blob,
    witnesses blob
)"
)]
pub async fn create_transaction_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_cell(
    id bigint PRIMARY KEY,
    tx_hash blob NOT NULL,
    output_index smallint NOT NULL,
    tx_index smallint NOT NULL,
    block_hash blob NOT NULL,
    block_number int NOT NULL,
    epoch_number blob NOT NULL,
    capacity bigint NOT NULL,
    lock_hash blob,
    lock_code_hash blob,
    lock_args blob,
    lock_script_type smallint,
    type_hash blob,
    type_code_hash blob,
    type_args blob,
    type_script_type smallint,
    data blob,
    is_data_complete bool,
    consumed_block_number int,
    consumed_block_hash blob,
    consumed_tx_hash blob,
    consumed_tx_index smallint,
    input_index smallint,
    since bigint
)"
)]
pub async fn create_cell_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_live_cell(
    id bigint PRIMARY KEY,
    output_index smallint NOT NULL,
    tx_hash blob NOT NULL,
    tx_index smallint NOT NULL,
    block_hash blob NOT NULL,
    block_number int NOT NULL,
    epoch_number blob NOT NULL,
    capacity bigint NOT NULL,
    lock_hash blob,
    lock_code_hash blob,
    lock_script_hash blob,
    lock_args blob,
    lock_script_type smallint,
    type_hash blob,
    type_code_hash blob,
    type_args blob,
    type_script_type smallint,
    data blob,
    is_data_complete bool
)"
)]
pub async fn create_live_cell_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_script(
    id bigint PRIMARY KEY,
    script_hash blob NOT NULL,
    script_hash_160 blob NOT NULL,
    script_code_hash blob NOT NULL,
    script_args blob,
    script_type smallint NOT NULL,
    script_args_len smallint
)"
)]
pub async fn create_script_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_uncle_relationship(
    block_hash blob,
    uncle_hashes blob,
    PRIMARY KEY(block_hash, uncle_hashes)
)"
)]
pub async fn create_uncle_relationship_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[cfg(test)]
#[sql(
    tx,
    "CREATE TABLE mercury_canonical_chain(
    block_number int PRIMARY KEY,
    block_hash blob NOT NULL
)"
)]
pub async fn create_canonical_chain_table(tx: &mut RBatisTxExecutor<'_>) -> () {}
