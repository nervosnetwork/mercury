mod fetch_mod_test;
mod get_block_test;
mod get_cell_test;
mod get_historical_live_cells_test;
mod get_tx_test;
mod other_test;
mod single_sql_test;

use crate::relational::fetch::rb_bytes_to_h256;
use crate::relational::{sql, to_rb_bytes, DBDriver, PaginationRequest};
use crate::{relational::RelationalStorage, Storage};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::core::ScriptHashType;
use ckb_types::{bytes::Bytes, core::BlockView, h160, packed, prelude::*, H160, H256};
use common::{Context, Order, Range};

use std::str::FromStr;

const MEMORY_DB: &str = ":memory:";
const POSTGRES_DB: &str = "127.0.0.1";
const BLOCK_DIR: &str = "../../devtools/test_data/blocks/";

pub async fn connect_pg_pool() -> RelationalStorage {
    init_debugger(true);
    let pool = RelationalStorage::new(0, 0, 100, 0, 60, 1800, 30, log::LevelFilter::Debug);
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
    let pool = RelationalStorage::new(0, 0, 100, 0, 60, 1800, 30, log::LevelFilter::Info);
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

fn caculate_lock_hash(code_hash: &str, args: &str, script_hash_type: ScriptHashType) -> H256 {
    let code_hash = H256::from_str(code_hash).unwrap();
    let args = H160::from_str(args).unwrap();
    let script = packed::Script::new_builder()
        .hash_type(script_hash_type.into())
        .code_hash(code_hash.pack())
        .args(ckb_types::bytes::Bytes::from(args.as_bytes().to_owned()).pack())
        .build();
    script.calc_script_hash().unpack()
}
