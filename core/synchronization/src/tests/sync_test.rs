use super::*;

use crate::table::to_rb_bytes;
use crate::Synchronization;

use common::Context;
use core_rpc_types::SyncState;
use core_storage::Storage;

use ckb_types::bytes::Bytes;
use ckb_types::prelude::Unpack;
use ckb_types::H256;
use parking_lot::RwLock;

use std::str::FromStr;
use std::sync::Arc;

#[tokio::test]
async fn test_sync() {
    let res = connect_and_create_tables().await;
    assert!(res.is_ok());

    let storage = res.unwrap();
    let sync_handler = Synchronization::new(
        storage.clone().inner(),
        storage.get_pool(),
        Arc::new(CkbRpcTestClient),
        4,
        9,
        Arc::new(RwLock::new(SyncState::ReadOnly)),
    );
    sync_handler.do_sync().await.unwrap();
    sync_handler.build_indexer_cell_table().await.unwrap();

    let pool = storage.get_pool();
    assert_eq!(10, pool.fetch_count("mercury_block").await.unwrap());
    assert_eq!(11, pool.fetch_count("mercury_transaction").await.unwrap());
    assert_eq!(12, pool.fetch_count("mercury_cell").await.unwrap());
    assert_eq!(11, pool.fetch_count("mercury_live_cell").await.unwrap());
    assert_eq!(13, pool.fetch_count("mercury_indexer_cell").await.unwrap());

    // During parallel synchronization, H256::default() will be added to the script table as the script hash of typescript,
    // so there will be one more than normal serial synchronization (append_block).
    assert_eq!(10, pool.fetch_count("mercury_script").await.unwrap());

    assert_eq!(
        10,
        pool.fetch_count("mercury_canonical_chain").await.unwrap()
    );
    assert_eq!(
        0,
        pool.fetch_count("mercury_registered_address")
            .await
            .unwrap()
    );
    assert_eq!(10, pool.fetch_count("mercury_sync_status").await.unwrap());
    assert_eq!(0, pool.fetch_count("mercury_in_update").await.unwrap());

    let block_hash =
        H256::from_str("10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606").unwrap();
    let res_block = storage
        .get_block(Context::new(), Some(block_hash.clone()), None)
        .await
        .unwrap();
    let res_block_hash: H256 = res_block.hash().unpack();
    assert_eq!(block_hash, res_block_hash);
}

#[tokio::test]
async fn test_to_rb_bytes() {
    let tx_hash = hex::decode("63000000000000000000000000000000").unwrap();
    let ret_rbatis_bytes = to_rb_bytes(&tx_hash);
    let ret_bytes = Bytes::from(tx_hash);
    assert_eq!(ret_rbatis_bytes.len(), ret_bytes.len());
}
