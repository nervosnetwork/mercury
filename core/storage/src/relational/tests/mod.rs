mod get_block_test;
mod get_cell_test;
mod get_tx_test;
mod other_test;
mod single_sql_test;

use crate::relational::fetch::bson_to_h256;
use crate::relational::{to_bson_bytes, DBDriver, PaginationRequest, XSQLPool};
use crate::{relational::RelationalStorage, Storage};

use common::{Order, Range};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::core::BlockView;
use ckb_types::{h160, prelude::*, H160, H256};

const MEMORY_DB: &str = ":memory:";
const POSTGRES_DB: &str = "127.0.0.1";
const BLOCK_DIR: &str = "../../devtools/test_data/blocks/";

lazy_static::lazy_static! {
    static ref TEST_POOL: RelationalStorage = RelationalStorage::new(100, 0, 0, log::LevelFilter::Info);
}

pub async fn connect_pg_pool() -> XSQLPool {
    init_debugger();
    let pool = XSQLPool::new(100, 0, 0, log::LevelFilter::Debug);
    pool.connect(
        DBDriver::PostgreSQL,
        "mercury",
        POSTGRES_DB,
        8432,
        "postgres",
        "123456",
    )
    .await
    .unwrap();

    pool
}

fn init_debugger() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
}

async fn connect_sqlite() {
    init_debugger();
    TEST_POOL
        .connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await
        .unwrap();
}

pub fn read_block_view(number: u64, dir_path: String) -> JsonBlockView {
    let file_name = number.to_string() + ".json";
    let path = dir_path + file_name.as_str();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

async fn connect_and_insert_blocks() {
    connect_sqlite().await;
    let mut tx = TEST_POOL.pool.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let data_path = String::from(BLOCK_DIR);
    for i in 0..10 {
        TEST_POOL
            .append_block(read_block_view(i, data_path.clone()).into())
            .await
            .unwrap();
    }
}
