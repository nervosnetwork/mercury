use common::anyhow::Result;
use sqlx::any::Any;
use sqlx::pool::PoolConnection;

use db_xsql::rbatis::executor::RBatisTxExecutor;
use db_xsql::rbatis::sql;

#[sql(
    tx,
    "UPDATE mercury_cell AS cell 
    SET consumed_block_number = consume.consumed_block_number,
    consumed_block_hash = consume.consumed_block_hash,
    consumed_tx_index = consume.consumed_tx_index,
    consumed_tx_hash = consume.consumed_tx_hash,
    input_index = consume.input_index,
    since = consume.since
    FROM mercury_consume_info AS consume 
    WHERE consume.consumed_block_number >= $1 AND consume.consumed_block_number < $2 AND consume.tx_hash = cell.tx_hash AND consume.output_index = cell.output_index"
)]
pub async fn update_cell_table(tx: &mut RBatisTxExecutor<'_>, from: &u32, to: &u32) -> () {}

#[sql(
    tx,
    "INSERT INTO mercury_live_cell (id, tx_hash, output_index, tx_index, block_hash, block_number, epoch_number, epoch_index, epoch_length, capacity, lock_hash, lock_code_hash, lock_args, lock_script_type, type_hash, type_code_hash, type_args, type_script_type, data)
	SELECT cell.id, cell.tx_hash, cell.output_index, cell.tx_index, cell.block_hash, cell.block_number, cell.epoch_number, cell.epoch_index, cell.epoch_length, cell.capacity, cell.lock_hash, cell.lock_code_hash, cell.lock_args, cell.lock_script_type, cell.type_hash, cell.type_code_hash, cell.type_args, cell.type_script_type, cell.data
  	FROM mercury_cell AS cell
	WHERE cell.block_number >= $1 AND cell.block_number < $2 AND cell.consumed_block_number IS NULL"
)]
pub async fn insert_into_live_cell(tx: &mut RBatisTxExecutor<'_>, from: &u32, to: &u32) -> () {}

#[sql(
    tx,
    "INSERT INTO mercury_script(script_hash, script_hash_160, script_code_hash, script_args, script_type, script_args_len)
    SELECT DISTINCT cell.script_hash, cell.script_hash_160, cell.script_code_hash, cell.script_args, cell.script_type, cell.script_args_len
    FROM(SELECT DISTINCT cell_lock.lock_hash AS script_hash, SUBSTRING(cell_lock.lock_hash, 1, 20) AS script_hash_160, cell_lock.lock_code_hash AS script_code_hash, cell_lock.lock_args AS script_args, cell_lock.lock_script_type AS script_type, LENGTH(cell_lock.lock_args) AS script_args_len 
    FROM mercury_cell AS cell_lock UNION ALL 
    SELECT DISTINCT cell_type.type_hash AS script_hash, SUBSTRING(cell_type.type_hash, 1, 20) AS script_hash_160, cell_type.type_code_hash AS script_code_hash, cell_type.type_args AS script_args, cell_type.type_script_type AS script_type, LENGTH(cell_type.type_args) AS script_args_len 
    FROM mercury_cell AS cell_type) AS cell"
)]
pub async fn insert_into_script(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[sql(tx, "DROP TABLE mercury_live_cell")]
pub async fn drop_live_cell_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[sql(tx, "DROP TABLE mercury_script")]
pub async fn drop_script_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[sql(tx, "DROP TABLE mercury_consume_info")]
pub async fn drop_consume_info_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[sql(
    tx,
    "
    CREATE TABLE mercury_live_cell(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    output_index int NOT NULL,
    tx_index int NOT NULL,
    block_hash bytea NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
    capacity bigint NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_args bytea,
    lock_script_type smallint,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type smallint,
    data bytea
    );
    CREATE INDEX \"index_live_cell_table_block_hash\" ON \"mercury_live_cell\" (\"block_hash\");
    CREATE INDEX \"index_live_cell_table_block_number\" ON \"mercury_live_cell\" (\"block_number\"); 
    CREATE INDEX \"index_live_cell_table_tx_hash_and_output_index\" ON \"mercury_live_cell\" (\"tx_hash\", \"output_index\");
    CREATE INDEX \"index_live_cell_table_lock_hash\" ON \"mercury_live_cell\" (\"lock_hash\");
    CREATE INDEX \"index_live_cell_table_lock_code_hash_and_lock_script_type\" ON \"mercury_live_cell\" (\"lock_code_hash\", \"lock_script_type\");
    CREATE INDEX \"index_live_cell_table_type_code_hash_and_type_script_type\" ON \"mercury_live_cell\" (\"type_code_hash\", \"type_script_type\");
    "
)]
pub async fn create_live_cell_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

#[sql(
    tx,
    "
    CREATE TABLE mercury_script(
        script_hash bytea NOT NULL PRIMARY KEY,
        script_hash_160 bytea NOT NULL,
        script_code_hash bytea NOT NULL,
        script_args bytea,
        script_type smallint NOT NULL,
        script_args_len int
    );
    CREATE INDEX \"index_script_table_script_hash\" ON \"mercury_script\" (\"script_hash\");
    CREATE INDEX \"index_script_table_code_hash\" ON \"mercury_script\" (\"script_code_hash\");
    CREATE INDEX \"index_script_table_args\" ON \"mercury_script\" (\"script_args\");
    CREATE INDEX \"index_script_table_script_hash_160\" ON \"mercury_script\" (\"script_hash_160\");
    "
)]
pub async fn create_script_table(tx: &mut RBatisTxExecutor<'_>) -> () {}

pub async fn create_consume_info_table(conn: &mut PoolConnection<Any>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mercury_consume_info(
            tx_hash bytea NOT NULL,
            output_index int NOT NULL,
            consumed_block_number bigint NOT NULL,
            consumed_block_hash bytea NOT NULL,
            consumed_tx_hash bytea NOT NULL,
            consumed_tx_index int NOT NULL,
            input_index int NOT NULL,
            since bytea NOT NULL,
            PRIMARY KEY(tx_hash, output_index)
        )",
    )
    .execute(conn)
    .await
    .map(|_| ())
    .map_err(Into::into)
}
