use core_storage::relational::table::BsonBytes;
use db_xsql::rbatis::{executor::RBatisConnExecutor, sql};

#[sql(
    conn,
    "INSERT INTO mercury_live_cell (id, tx_hash, output_index, tx_index, block_hash, block_number, epoch_number, epoch_index, epoch_length, capacity, lock_hash, lock_code_hash, lock_args, lock_script_type, type_hash, type_code_hash, type_args, type_script_type, data)
	SELECT cell.id, cell.tx_hash, cell.output_index, cell.tx_index, cell.block_hash, cell.block_number, cell.epoch_number, cell.epoch_index, cell.epoch_length, cell.capacity, cell.lock_hash, cell.lock_code_hash, cell.lock_args, cell.lock_script_type, cell.type_hash, cell.type_code_hash, cell.type_args, cell.type_script_type, cell.data
  	FROM mercury_cell AS cell LEFT JOIN mercury_consume_info AS consume ON cell.tx_hash = consume.tx_hash AND cell.output_index = consume.output_index 
	WHERE consume.consumed_block_hash IS NULL"
)]
pub async fn insert_into_live_cell(conn: &mut RBatisConnExecutor<'_>) -> () {}

#[sql(conn, "SELECT script_hash::bytea from mercury_script")]
pub async fn fetch_exist_script_hash(conn: &mut RBatisConnExecutor<'_>) -> Vec<BsonBytes> {}

#[sql(conn, "DROP TABLE mercury_live_cell")]
pub async fn drop_live_cell_table(conn: &mut RBatisConnExecutor<'_>) -> () {}

#[sql(
    conn,
    "CREATE TABLE mercury_live_cell(
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
)"
)]
pub async fn create_live_cell_table(conn: &mut RBatisConnExecutor<'_>) -> () {}
