use db_xsql::rbatis::{executor::RBatisConnExecutor, sql};

#[sql(
    conn,
    "insert into mercury_live_cell (id, tx_hash, output_index, tx_index, block_hash, block_number, epoch_number, epoch_index, epoch_length, capacity, lock_hash, lock_code_hash, lock_args, lock_script_type, type_hash, type_code_hash, type_args, type_script_type, data) 
	select id, tx_hash, output_index, tx_index, block_hash, block_number, epoch_number, epoch_index, epoch_length, capacity, lock_hash, lock_code_hash, lock_args, lock_script_type, type_hash, type_code_hash, type_args, type_script_type, data 
	from mercury_cell cell left join mercury_consume_info consume on cell.tx_hash = consume.tx_hash and cell.output_index = consume.output_index
	where consume.consumed_block_hash is null"
)]
pub async fn insert_into_live_cell(conn: &mut RBatisConnExecutor<'_>) -> () {}
