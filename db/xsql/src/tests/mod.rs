use crate::{DBAdapter, DBDriver, PaginationRequest, XSQLPool};

use common::{anyhow::Result, async_trait, Order, Range};

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use ckb_types::core::BlockView;
use db_protocol::DB;
use tokio::test;

const MEMORY_DB: &str = ":memory:";
const BLOCK_DIR: &str = "src/tests/blocks/";

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

fn init_debugger() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
}

fn read_block_view(number: u64, dir_path: String) -> JsonBlockView {
    let file_name = number.to_string() + ".json";
    let path = dir_path + file_name.as_str();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

async fn connect_sqlite() {
    init_debugger();
    TEST_POOL
        .connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await
        .unwrap();
}

async fn connect_and_insert_blocks() {
    connect_sqlite().await;
    TEST_POOL.create_tables().await.unwrap();

    let data_path = String::from(BLOCK_DIR);
    for i in 0..10 {
        TEST_POOL
            .append_block(read_block_view(i, data_path.clone()).into())
            .await
            .unwrap();
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
async fn test_get_block_header_by_number() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL.get_block_header(None, Some(0)).await.unwrap();
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
    assert_eq!(block.header(), res);
}

#[test]
async fn test_get_block_by_number() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL.get_block(None, Some(0)).await.unwrap();
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
    let block = block.as_advanced_builder().set_uncles(vec![]).build();
    assert_eq!(block.data(), res.data());
}

#[test]
async fn test_get_tx() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL
        .get_transactions(
            vec![],
            vec![],
            vec![],
            Some(Range::new(0, 10)),
            PaginationRequest::new(0, Order::Asc, Some(20), None),
        )
        .await
        .unwrap();
    println!("{:?}", res.response.len());
}

#[test]
async fn test_get_live_cells() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL
        .get_live_cells(
            vec![],
            vec![],
            Some(0),
            None,
            PaginationRequest::new(0, Order::Asc, Some(20), None),
        )
        .await
        .unwrap();
    println!("{:?}", res.response.len());
}
