use crate::{DBAdapter, DBDriver, XSQLPool};

use common::{anyhow::Result, async_trait};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::core::BlockView;
use db_protocol::DB;
use tokio::test;

lazy_static::lazy_static! {
    static ref TEST_POOL: XSQLPool<MockClient> = XSQLPool::new(MockClient {}, 100, 0, 0);
}

#[derive(Default, Clone, Debug)]
struct MockClient;

#[async_trait]
impl DBAdapter for MockClient {
    async fn pull_blocks(&self, _: Vec<u64>) -> Result<Vec<BlockView>> {
        unreachable!()
    }
}

async fn connect_sqlite() {
    TEST_POOL
        .connect(
            DBDriver::SQLite,
            "../../free-space/sqlite/test.db",
            "",
            0,
            "",
            "",
        )
        .await
        .unwrap();
}

async fn connect_and_insert_blocks() {
    connect_sqlite().await;
    let data_path = String::from("src/tests/blocks/");
    println!("{:?}", std::env::current_dir().unwrap());

    for i in 0..10 {
        let file_name = i.to_string() + ".json";
        let path = data_path.clone() + file_name.as_str();
        let block: JsonBlockView = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();

        TEST_POOL.append_block(block.into()).await.unwrap();
    }
}

#[test]
async fn test_insert() {
    connect_and_insert_blocks().await;
}

#[test]
async fn test_remove_all() {
    connect_sqlite().await;
    TEST_POOL.delete_all().await.unwrap();
}
