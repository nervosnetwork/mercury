mod get_block_test;
mod get_cell_test;
mod get_tx_test;
mod other_test;
mod single_sql_test;

use crate::relational::fetch::rb_bytes_to_h256;
use crate::relational::{sql, to_rb_bytes, DBDriver, PaginationRequest, XSQLPool};
use crate::{relational::RelationalStorage, Storage};

use common::{Context, Order, Range};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::{bytes::Bytes, core::BlockView, h160, prelude::*, H256};

const MEMORY_DB: &str = ":memory:";
const POSTGRES_DB: &str = "127.0.0.1";
const BLOCK_DIR: &str = "../../devtools/test_data/blocks/";

pub async fn connect_pg_pool() -> XSQLPool {
    init_debugger(true);
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

fn init_debugger(option: bool) {
    if option {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }
}

async fn connect_sqlite() -> RelationalStorage {
    init_debugger(false);
    let pool = RelationalStorage::new(100, 0, 0, log::LevelFilter::Info);
    pool.connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await
        .unwrap();
    pool
}

pub fn read_block_view(number: u64, dir_path: String) -> JsonBlockView {
    let file_name = number.to_string() + ".json";
    let path = dir_path + file_name.as_str();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

async fn connect_and_insert_blocks() -> RelationalStorage {
    let pool = connect_sqlite().await;
    let mut tx = pool.pool.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let data_path = String::from(BLOCK_DIR);
    for i in 0..10 {
        pool.append_block(Context::new(), read_block_view(i, data_path.clone()).into())
            .await
            .unwrap();
    }
    pool
}
