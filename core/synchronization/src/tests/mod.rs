mod sync_test;

use crate::SyncAdapter;

use common::{async_trait, Result};
use core_storage::{relational::RelationalStorage, DBDriver};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::core::{BlockNumber, BlockView};

const MEMORY_DB: &str = ":memory:";
const BLOCK_DIR: &str = "../../devtools/test_data/blocks/";

async fn connect_sqlite() -> Result<RelationalStorage> {
    let mut pool = RelationalStorage::new(0, 0, 100, 0, 60, 1800, 30, log::LevelFilter::Info);
    pool.connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await?;
    Ok(pool)
}

async fn connect_and_create_tables() -> Result<RelationalStorage> {
    let pool = connect_sqlite().await?;
    let tx = pool.sqlx_pool.transaction().await?;
    xsql_test::create_tables(tx).await?;
    Ok(pool)
}

pub fn read_block_view(number: u64, dir_path: String) -> JsonBlockView {
    let file_name = number.to_string() + ".json";
    let path = dir_path + file_name.as_str();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[derive(Clone, Debug)]
pub struct CkbRpcTestClient;

#[async_trait]
impl SyncAdapter for CkbRpcTestClient {
    async fn pull_blocks(&self, _block_numbers: Vec<BlockNumber>) -> Result<Vec<BlockView>> {
        let ret = (0..10)
            .map(|i| {
                let block_view: BlockView = read_block_view(i, String::from(BLOCK_DIR)).into();
                block_view
            })
            .into_iter()
            .collect();
        Ok(ret)
    }
}
