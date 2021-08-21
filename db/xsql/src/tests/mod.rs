use crate::{DBAdapter, DBDriver, PaginationRequest, XSQLPool};

use common::{anyhow::Result, async_trait, Order};

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
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    TEST_POOL
        .connect(DBDriver::SQLite, ":memory:", "", 0, "", "")
        .await
        .unwrap();
}

async fn connect_and_insert_blocks() {
    connect_sqlite().await;
    TEST_POOL.create_tables().await.unwrap();

    let data_path = String::from("src/tests/blocks/");
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
    connect_and_insert_blocks().await;
    TEST_POOL.delete_all_data().await.unwrap();
}

#[test]
async fn test_get_block() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL
        .get_live_cells(
            vec![],
            vec![],
            Some(0),
            None,
            PaginationRequest::new(0, Order::Asc, None, None),
        )
        .await
        .unwrap();
    println!("{:?}", res);
}
