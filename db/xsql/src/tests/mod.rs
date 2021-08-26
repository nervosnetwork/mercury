use crate::{DBAdapter, DBDriver, PaginationRequest, XSQLPool};

use common::{anyhow::Result, async_trait, Order, Range};

use ckb_types::core::BlockView;
use ckb_types::{h160, prelude::*, H160};
use db_protocol::DB;
use tokio::test;

use std::sync::Arc;

const MEMORY_DB: &str = ":memory:";
const BLOCK_DIR: &str = "src/tests/blocks/";

lazy_static::lazy_static! {
    static ref TEST_POOL: XSQLPool<MockClient> = XSQLPool::new(Arc::new(MockClient), 100, 0, 0);
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

async fn connect_sqlite() {
    init_debugger();
    TEST_POOL
        .connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await
        .unwrap();
}

async fn connect_and_insert_blocks() {
    connect_sqlite().await;
    let mut tx = TEST_POOL.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let data_path = String::from(BLOCK_DIR);
    for i in 0..10 {
        TEST_POOL
            .append_block(xsql_test::read_block_view(i, data_path.clone()).into())
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
    let mut tx = TEST_POOL.transaction().await.unwrap();
    xsql_test::delete_all_data(&mut tx).await.unwrap();
}

#[test]
async fn test_get_block_header_by_number() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL.get_block_header(None, Some(0)).await.unwrap();
    let block: BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string()).into();
    assert_eq!(block.header(), res);
}

#[test]
async fn test_get_block_by_number() {
    connect_and_insert_blocks().await;
    let res = TEST_POOL.get_block(None, Some(0)).await.unwrap();
    let block: BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string()).into();
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
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
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
            None,
            vec![],
            vec![],
            Some(0),
            None,
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap();
    println!("{:?}", res.response.len());
}

#[test]
async fn test_register_addresses() {
    connect_sqlite().await;
    let mut tx = TEST_POOL.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let lock_hash = h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64");
    let address = String::from("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v");
    let addresses = vec![(lock_hash.clone(), address.clone())];
    let res = TEST_POOL
        .register_addresses(addresses.clone())
        .await
        .unwrap();
    assert_eq!(res[0], lock_hash);
    let res = TEST_POOL.get_registered_address(lock_hash).await.unwrap();
    assert_eq!(res, Some(address));
}

#[test]
async fn test_get_db_info() {
    connect_sqlite().await;
    let res = TEST_POOL.get_db_info().unwrap();
    assert_eq!(res.version, clap::crate_version!().to_string());
    assert_eq!(res.db, DBDriver::PostgreSQL);
    assert_eq!(res.center_id, 0);
    assert_eq!(res.machine_id, 0);
    assert_eq!(res.conn_size, 100);
}

#[test]
async fn test_get_spent_transaction_hash() {
    connect_and_insert_blocks().await;
    let block: BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string()).into();
    let tx = &block.transaction(0).unwrap();
    let outpoint = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx.hash().unpack(), // 0xb50ef2272f9f72b11e21ec12bd1b8fc9136cafc25c197b6fd4c2eb4b19fa905c
        index: 0u32.into(),
    };
    let res = TEST_POOL
        .get_spent_transaction_hash(outpoint.into())
        .await
        .unwrap();
    assert_eq!(res, None)
}
