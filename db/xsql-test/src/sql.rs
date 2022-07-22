use common::anyhow::Result;
use sqlx::{Any, Transaction};

pub async fn delete_block_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_block")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_transaction_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_transaction")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_cell_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_cell")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_consume_info_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_consume_info")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_live_cell_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_live_cell")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_indexer_cell_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_indexer_cell")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_script_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_script")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_canonical_chain_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_canonical_chain")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_registered_address_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_registered_address")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn delete_sync_status_table_data(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query("DELETE FROM mercury_sync_status")
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn create_block_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_block(
        block_hash blob PRIMARY KEY,
        block_number int NOT NULL,
        version smallint NOT NULL,
        compact_target int NOT NULL,
        block_timestamp bigint NOT NULL,
        epoch_number int NOT NULL,
        epoch_index smallint NOT NULL,
        epoch_length smallint NOT NULL,
        parent_hash blob NOT NULL,
        transactions_root blob NOT NULL,
        proposals_hash blob NOT NULL,
        uncles_hash blob,
        uncles blob,
        uncles_count int,
        dao blob NOT NULL,
        nonce blob NOT NULL,
        proposals blob
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_transaction_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
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
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_cell_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_cell(
        id bigint PRIMARY KEY,
        tx_hash blob NOT NULL,
        output_index smallint NOT NULL,
        tx_index smallint NOT NULL,
        block_hash blob NOT NULL,
        block_number bigint NOT NULL,
        epoch_number bigint NOT NULL,
        epoch_index bigint NOT NULL,
        epoch_length bigint NOT NULL,
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
        consumed_block_number bigint,
        consumed_block_hash blob,
        consumed_tx_hash blob,
        consumed_tx_index int,
        input_index int,
        since blob
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_consume_info_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_consume_info(
        tx_hash blob NOT NULL,
        output_index int NOT NULL,
        consumed_block_number bigint NOT NULL,
        consumed_block_hash blob NOT NULL,
        consumed_tx_hash blob NOT NULL,
        consumed_tx_index int NOT NULL,
        input_index int NOT NULL,
        since blob NOT NULL,
        PRIMARY KEY(tx_hash, output_index)
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_live_cell_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_live_cell(
        id bigint PRIMARY KEY,
        output_index smallint NOT NULL,
        tx_hash blob NOT NULL,
        tx_index smallint NOT NULL,
        block_hash blob NOT NULL,
        block_number bigint NOT NULL,
        epoch_number bigint NOT NULL,
        epoch_index bigint NOT NULL,
        epoch_length bigint NOT NULL,
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
        data blob
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_indexer_cell_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_indexer_cell(
        id bigint PRIMARY KEY,
        block_number int NOT NULL,
        io_type smallint NOT NULL,
        io_index int NOT NULL,
        tx_hash blob NOT NULL,
        tx_index int NOT NULL,
        lock_hash blob,
        lock_code_hash blob,
        lock_args blob,
        lock_script_type smallint,
        type_hash blob,
        type_code_hash blob,
        type_args blob,
        type_script_type smallint
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_script_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_script(
        script_hash blob NOT NULL PRIMARY KEY,
        script_hash_160 blob NOT NULL,
        script_code_hash blob NOT NULL,
        script_args blob,
        script_type smallint NOT NULL,
        script_args_len smallint
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_canonical_chain_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_canonical_chain(
        block_number bigint PRIMARY KEY,
        block_hash blob NOT NULL
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_registered_address_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_registered_address(
        lock_hash blob NOT NULL PRIMARY KEY,
        address varchar NOT NULL
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

pub async fn create_sync_status_table(tx: &mut Transaction<'_, Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE mercury_sync_status(
        block_number int NOT NULL PRIMARY KEY
    )",
    )
    .execute(&mut *tx)
    .await
    .map(|_| ())
    .map_err(Into::into)
}
