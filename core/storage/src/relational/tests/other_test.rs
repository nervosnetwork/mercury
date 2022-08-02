use super::*;

#[tokio::test]
async fn test_insert() {
    let storage = connect_and_insert_blocks().await;

    let pool = storage.get_pool();
    assert_eq!(10, pool.fetch_count("mercury_block").await.unwrap());
    assert_eq!(11, pool.fetch_count("mercury_transaction").await.unwrap());
    assert_eq!(12, pool.fetch_count("mercury_cell").await.unwrap());
    assert_eq!(11, pool.fetch_count("mercury_live_cell").await.unwrap());
    assert_eq!(13, pool.fetch_count("mercury_indexer_cell").await.unwrap());
    assert_eq!(9, pool.fetch_count("mercury_script").await.unwrap());
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
}

#[tokio::test]
async fn test_remove_all() {
    let storage = connect_and_insert_blocks().await;

    let tx = storage.sqlx_pool.transaction().await.unwrap();
    xsql_test::delete_all_data(tx).await.unwrap();

    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_block")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_transaction")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage.sqlx_pool.fetch_count("mercury_cell").await.unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_live_cell")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_indexer_cell")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_script")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_canonical_chain")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_register_addresses() {
    let pool = connect_sqlite().await;
    let tx = pool.sqlx_pool.transaction().await.unwrap();
    xsql_test::create_tables(tx).await.unwrap();

    let lock_hash = h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64");
    let address = String::from("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v");
    let addresses = vec![(lock_hash.clone(), address.clone())];
    let res = pool
        .register_addresses(Context::new(), addresses.clone())
        .await
        .unwrap();
    assert_eq!(res[0], lock_hash);
    let res = pool
        .get_registered_address(Context::new(), lock_hash)
        .await
        .unwrap();
    assert_eq!(res, Some(address));
}

#[tokio::test]
async fn test_get_db_info() {
    let pool = connect_sqlite().await;
    let res = pool.get_db_info(Context::new()).unwrap();
    assert_eq!(res.version, clap::crate_version!().to_string());
    assert_eq!(res.db, DBDriver::PostgreSQL);
    assert_eq!(res.center_id, 0);
    assert_eq!(res.machine_id, 0);
    assert_eq!(res.conn_size, 100);
}

#[tokio::test]
async fn test_get_tx_hash() {
    let pool = connect_and_insert_blocks().await;
    let block_hash =
        hex::decode("bc5969d7829ea32aca5784a9eb94cf219f084d2451706bec378f08e23c417ce3").unwrap();
    let res = pool
        .query_transaction_hashes_by_block_hash(&block_hash)
        .await
        .unwrap();
    assert!(res.is_empty());
    let block_hash =
        hex::decode("10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606").unwrap();
    let res = pool
        .query_transaction_hashes_by_block_hash(&block_hash)
        .await
        .unwrap();
    assert_eq!(2, res.len());
}

#[tokio::test]
async fn test_rollback_block() {
    let storage = connect_sqlite().await;

    let tx = storage.sqlx_pool.transaction().await.unwrap();
    xsql_test::create_tables(tx).await.unwrap();

    let data_path = String::from(BLOCK_DIR);
    storage
        .append_block(read_block_view(0, data_path.clone()).into())
        .await
        .unwrap();

    assert_eq!(
        1,
        storage
            .sqlx_pool
            .fetch_count("mercury_block")
            .await
            .unwrap()
    );
    assert_eq!(
        1,
        storage
            .sqlx_pool
            .fetch_count("mercury_sync_status")
            .await
            .unwrap()
    );
    assert_eq!(
        1,
        storage
            .sqlx_pool
            .fetch_count("mercury_canonical_chain")
            .await
            .unwrap()
    );
    assert_eq!(
        2,
        storage
            .sqlx_pool
            .fetch_count("mercury_transaction")
            .await
            .unwrap()
    );
    assert_eq!(
        12,
        storage.sqlx_pool.fetch_count("mercury_cell").await.unwrap()
    );
    assert_eq!(
        11,
        storage
            .sqlx_pool
            .fetch_count("mercury_live_cell")
            .await
            .unwrap()
    );
    assert_eq!(
        9,
        storage
            .sqlx_pool
            .fetch_count("mercury_script")
            .await
            .unwrap()
    );
    assert_eq!(
        13,
        storage
            .sqlx_pool
            .fetch_count("mercury_indexer_cell")
            .await
            .unwrap()
    );

    let block_hash =
        H256::from_str("10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606").unwrap();
    storage.rollback_block(0, block_hash).await.unwrap();

    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_block")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_sync_status")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_canonical_chain")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_transaction")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage.sqlx_pool.fetch_count("mercury_cell").await.unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_live_cell")
            .await
            .unwrap()
    );
    assert_eq!(
        9,
        storage
            .sqlx_pool
            .fetch_count("mercury_script")
            .await
            .unwrap()
    );
    assert_eq!(
        0,
        storage
            .sqlx_pool
            .fetch_count("mercury_indexer_cell")
            .await
            .unwrap()
    );
}
