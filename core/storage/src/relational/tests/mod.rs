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
    let block: ckb_jsonrpc_types::BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string());
    let block_hash = &block.header.hash;
    println!("block hash is {:?}", block_hash.to_string()); // 10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606

    // from ckb_types::core::BlockView
    let block: ckb_types::core::BlockView = block.into();
    let block_hash: H256 = block.hash().unpack();
    let hash_from_header: H256 = block.header().hash().unpack();
    println!("block hash is {:?}", block_hash.to_string()); // 42a3b5f44b670a0ee2aedd5b84693e4fe4922f77f7e294e46f68fcbb07a8af72
    println!("hash from header is {:?}", hash_from_header.to_string()); // 42a3b5f44b670a0ee2aedd5b84693e4fe4922f77f7e294e46f68fcbb07a8af72

    // from block table
    let block_table = TEST_POOL.query_block_by_number(0).await.unwrap();
    println!(
        "hash in block table: {:?}",
        bson_to_h256(&block_table.block_hash).to_string() // 42a3b5f44b670a0ee2aedd5b84693e4fe4922f77f7e294e46f68fcbb07a8af72
    );

    // from built block view
    let res = TEST_POOL.get_block(None, Some(0)).await.unwrap();
    let block_hash: H256 = res.hash().unpack();
    println!("block hash is {:?}", block_hash.to_string()); // b4a5e07d2419e4d4ef1f80152e1cd83a457a8d2dd014a6f53e4fbc7bbc4b6a83
}

#[test]
async fn test_get_transaction_hash() {
    connect_and_insert_blocks().await;

    // from json deserialization
    let block: ckb_jsonrpc_types::BlockView = xsql_test::read_block_view(0, BLOCK_DIR.to_string());
    let txs = &block.transactions;
    println!("tx 0 hash: {:?}", txs[0].hash.to_string()); // 8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f
    println!("tx 1 hash: {:?}", txs[1].hash.to_string()); // f8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37

    // from ckb_types::core::BlockView
    let block: ckb_types::core::BlockView = block.into();
    let txs = block.transactions();
    let tx_0_hash: H256 = txs[0].hash().unpack();
    let tx_1_hash: H256 = txs[1].hash().unpack();
    println!("tx 0 hash: {:?}", tx_0_hash.to_string()); // b50ef2272f9f72b11e21ec12bd1b8fc9136cafc25c197b6fd4c2eb4b19fa905c
    println!("tx 1 hash: {:?}", tx_1_hash.to_string()); // f8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37

    // from tx table
    let block_table = TEST_POOL.query_block_by_number(0).await.unwrap();
    let _block_hash = bson_to_h256(&block_table.block_hash); // 42a3b5f44b670a0ee2aedd5b84693e4fe4922f77f7e294e46f68fcbb07a8af72
    let txs = TEST_POOL
        .query_transactions_by_block_hash(&block_table.block_hash)
        .await
        .unwrap();
    println!("tx 0 hash: {:?}", bson_to_h256(&txs[0].tx_hash).to_string()); // b50ef2272f9f72b11e21ec12bd1b8fc9136cafc25c197b6fd4c2eb4b19fa905c
    println!("tx 1 hash: {:?}", bson_to_h256(&txs[1].tx_hash).to_string()); // f8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37

    // from built tx view
    let txs = TEST_POOL
        .get_transactions(
            vec![],
            vec![],
            vec![],
            Some(Range::new(0, 0)),
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response;
    println!("txs len: {:?}", txs.len());
    let tx_0_hash: H256 = txs[0].hash().unpack();
    let tx_1_hash: H256 = txs[1].hash().unpack();
    println!("tx 0 hash: {:?}", tx_0_hash.to_string()); // b7dcc2f85695963b47230128872e6bc629ab3ec0350606157d1e7de54cc2daac
    println!("tx 1 hash: {:?}", tx_1_hash.to_string()); // 8c4eff26d72a9d24e5b357cc7d4c72adb39ff89aa2f50288c6ef1a1800025a2c

    // from built block view
    let res = TEST_POOL.get_block(None, Some(0)).await.unwrap();
    let txs = res.transactions();
    let tx_0_hash: H256 = txs[0].hash().unpack();
    let tx_1_hash: H256 = txs[1].hash().unpack();
    println!("from built block view");
    println!("tx 0 hash: {:?}", tx_0_hash.to_string()); // b7dcc2f85695963b47230128872e6bc629ab3ec0350606157d1e7de54cc2daac
    println!("tx 1 hash: {:?}", tx_1_hash.to_string()); // 8c4eff26d72a9d24e5b357cc7d4c72adb39ff89aa2f50288c6ef1a1800025a2c
}
