use crate::fetch::bson_to_h256;
use crate::{to_bson_bytes, DBAdapter, DBDriver, PaginationRequest, XSQLPool};

use common::{Result, async_trait, Order, Range};

use ckb_types::core::BlockView;
use ckb_types::{h160, prelude::*, H160, H256};
use db_protocol::DB;
use tokio::test;

use std::sync::Arc;

const MEMORY_DB: &str = ":memory:";
const BLOCK_DIR: &str = "src/tests/blocks/";

lazy_static::lazy_static! {
    static ref TEST_POOL: XSQLPool<MockClient> = XSQLPool::new(Arc::new(MockClient), 100, 0, 0, log::LevelFilter::Info);
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
async fn test_get_txs_from_genesis_block() {
    connect_and_insert_blocks().await;
    let txs_from_db: Vec<ckb_jsonrpc_types::TransactionView> = TEST_POOL
        .get_transactions(
            vec![],
            vec![],
            vec![],
            Some(Range::new(0, 0)),
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response
        .into_iter()
        .map(|tx| ckb_jsonrpc_types::TransactionView::from(tx))
        .collect();

    let block: ckb_jsonrpc_types::BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string());
    let txs_from_json: Vec<ckb_jsonrpc_types::TransactionView> = block.transactions;

    assert_eq!(txs_from_db[0].hash, txs_from_json[0].hash);
    assert_eq!(txs_from_db[1].hash, txs_from_json[1].hash);
}

#[test]
async fn test_get_txs_except_genesis_block() {
    connect_and_insert_blocks().await;
    let txs_from_db = TEST_POOL
        .get_transactions(
            vec![],
            vec![],
            vec![],
            Some(Range::new(1, 10)),
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response;
    let tx_hashes_from_db: Vec<H256> = txs_from_db
        .iter()
        .map(|tx| tx.hash().clone().unpack())
        .collect();

    let mut txs_from_json: Vec<ckb_jsonrpc_types::TransactionView> = vec![];
    for i in 1..10 {
        let block: ckb_jsonrpc_types::BlockView =
            xsql_test::read_block_view(i, BLOCK_DIR.to_string());
        let mut txs = block.transactions;
        txs_from_json.append(&mut txs);
    }
    let tx_hashes_from_json: Vec<H256> = txs_from_json.iter().map(|tx| tx.hash.clone()).collect();

    assert_eq!(tx_hashes_from_db, tx_hashes_from_json);
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

#[test]
async fn test_get_block_info() {
    connect_and_insert_blocks().await;
    let block_table = TEST_POOL.query_block_by_number(0).await.unwrap();
    let tx_tables = TEST_POOL
        .query_transactions_by_block_hash(&block_table.block_hash)
        .await
        .unwrap();
    let tx_hashes: Vec<H256> = tx_tables
        .iter()
        .map(|tx| bson_to_h256(&tx.tx_hash))
        .collect();

    let block_info = TEST_POOL.get_simple_block(None, Some(0)).await.unwrap();
    assert_eq!(
        block_table.block_hash,
        to_bson_bytes(&block_info.block_hash.as_bytes())
    );
    assert_eq!(tx_hashes, block_info.transactions);
}

#[test]
async fn test_get_block_hash() {
    connect_and_insert_blocks().await;

    // from json deserialization
    let block_from_json: ckb_jsonrpc_types::BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string());
    let block_hash_from_json = &block_from_json.header.hash;
    println!("block hash is {:?}", block_hash_from_json.to_string()); // 10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606

    // from ckb_types::core::BlockView
    let block_core_view: ckb_types::core::BlockView = block_from_json.clone().into();
    let block_hash_core_view: H256 = block_core_view.hash().unpack();
    let block_hash_from_header: H256 = block_core_view.header().hash().unpack();
    println!("block hash is {:?}", block_hash_core_view.to_string());
    println!("hash from header is {:?}", block_hash_from_header.to_string()); 

    assert_eq!(block_hash_core_view, block_hash_from_header);
    assert_eq!(block_hash_from_json, &block_hash_core_view);

    // from block table
    let block_table = TEST_POOL.query_block_by_number(0).await.unwrap();
    let block_hash_from_table = bson_to_h256(&block_table.block_hash);
    println!(
        "hash in block table: {:?}",
        block_hash_from_table.to_string() 
    );

    assert_eq!(block_hash_from_json, &block_hash_from_table);

    // from built block view
    let res = TEST_POOL.get_block(None, Some(0)).await.unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    println!("block hash is {:?}", block_built_hash.to_string()); // 

    assert_eq!(block_hash_from_json, &block_built_hash);
}
