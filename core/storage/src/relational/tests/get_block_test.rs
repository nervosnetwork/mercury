use super::*;

use sqlx::Row;

#[tokio::test]
async fn test_get_block_header_of_genesis() {
    let pool = connect_and_insert_blocks().await;
    let res = pool
        .get_block_header(Context::new(), None, Some(0))
        .await
        .unwrap();
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
    assert_eq!(block.header(), res);
}

#[tokio::test]
async fn test_get_block_header_by_number() {
    let pool = connect_and_insert_blocks().await;
    let res = pool
        .get_block_header(Context::new(), None, Some(1))
        .await
        .unwrap();
    let block: BlockView = read_block_view(1, BLOCK_DIR.to_string()).into();
    assert_eq!(block.header(), res);
}

#[tokio::test]
async fn test_get_simple_block() {
    let pool = connect_and_insert_blocks().await;
    let block_table = pool.query_block_by_number(0).await.unwrap();
    let block_hash = H256::from_slice(&block_table.get::<Vec<u8>, _>("block_hash")).unwrap();
    let tx_hashes = pool
        .query_transaction_hashes_by_block_hash(block_hash.as_bytes())
        .await
        .unwrap();
    let block_info = pool
        .get_simple_block(Context::new(), None, Some(0))
        .await
        .unwrap();
    assert_eq!(
        &block_table.get::<Vec<u8>, _>("block_hash"),
        block_info.block_hash.as_bytes()
    );
    assert_eq!(tx_hashes, block_info.transactions);
}

#[tokio::test]
async fn test_get_block_of_genesis() {
    let pool = connect_and_insert_blocks().await;

    // from json deserialization
    let block_from_json: ckb_jsonrpc_types::BlockView = read_block_view(0, BLOCK_DIR.to_string());
    let block_hash_from_json = block_from_json.header.hash.clone();
    println!("block hash is {:?}", block_hash_from_json.to_string());

    // from ckb_types::core::BlockView
    let block_core_view: ckb_types::core::BlockView = block_from_json.clone().into();
    let block_hash_core_view: H256 = block_core_view.hash().unpack();
    let block_hash_from_header: H256 = block_core_view.header().hash().unpack();
    println!("block hash is {:?}", block_hash_core_view.to_string());
    assert_eq!(block_hash_core_view, block_hash_from_header);
    assert_eq!(block_hash_from_json, block_hash_core_view);

    // from block table
    let block_table = pool.query_block_by_number(0).await.unwrap();
    let block_hash_from_table = bytes_to_h256(&block_table.get::<Vec<u8>, _>("block_hash"));
    println!(
        "hash in block table: {:?}",
        block_hash_from_table.to_string()
    );

    assert_eq!(block_hash_from_json, block_hash_from_table);

    // from built block view
    let res = pool.get_block(Context::new(), None, Some(0)).await.unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    println!("block hash is {:?}", block_built_hash.to_string());

    assert_eq!(block_hash_from_json, block_built_hash);
    assert_eq!(block_core_view.data(), res.data());
}

#[tokio::test]
async fn test_get_block_by_number() {
    let pool = connect_and_insert_blocks().await;

    // from json deserialization
    let block_from_json: ckb_jsonrpc_types::BlockView = read_block_view(1, BLOCK_DIR.to_string());
    let block_hash_from_json = block_from_json.header.hash.clone();
    println!("block hash is {:?}", block_hash_from_json.to_string());

    // from ckb_types::core::BlockView
    let block_core_view: ckb_types::core::BlockView = block_from_json.clone().into();
    let block_hash_core_view: H256 = block_core_view.hash().unpack();
    let block_hash_from_header: H256 = block_core_view.header().hash().unpack();
    println!("block hash is {:?}", block_hash_core_view.to_string());
    assert_eq!(block_hash_core_view, block_hash_from_header);
    assert_eq!(block_hash_from_json, block_hash_core_view);

    // from block table
    let block_table = pool.query_block_by_number(1).await.unwrap();
    let block_hash_from_table = bytes_to_h256(&block_table.get::<Vec<u8>, _>("block_hash"));
    println!(
        "hash in block table: {:?}",
        block_hash_from_table.to_string()
    );

    assert_eq!(block_hash_from_json, block_hash_from_table);

    // from built block view
    let count = pool.block_count(Context::new()).await.unwrap();
    let res = pool.get_block(Context::new(), None, Some(1)).await.unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    println!("block hash is {:?}", block_built_hash.to_string());

    assert_eq!(10, count);
    assert_eq!(block_hash_from_json, block_built_hash);
    assert_eq!(block_core_view.data(), res.data());
}

#[tokio::test]
async fn test_get_block_by_hash() {
    let pool = connect_and_insert_blocks().await;

    // from built block view
    let block_hash =
        H256::from_str("d5ac7cf8c34a975bf258a34f1c2507638487ab71aa4d10a9ec73704aa3abf9cd").unwrap();
    let res = pool
        .get_block(Context::new(), Some(block_hash.clone()), None)
        .await
        .unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    assert_eq!(block_hash, block_built_hash);

    // get tip
    let res = pool.get_block(Context::new(), None, None).await.unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    assert_eq!(
        "953761d56c03bfedf5e70dde0583470383184c41331f709df55d4acab5358640".to_string(),
        block_built_hash.to_string()
    );

    // query block hash and block number at the same time
    let block_number_9_hash =
        H256::from_str("953761d56c03bfedf5e70dde0583470383184c41331f709df55d4acab5358640").unwrap();
    let res_1 = pool
        .get_block(Context::new(), Some(block_number_9_hash.clone()), Some(9))
        .await;
    let res_2 = pool
        .get_block(Context::new(), Some(block_number_9_hash), Some(1))
        .await;
    assert!(res_1.is_ok());
    assert!(res_2.is_err());
}

#[tokio::test]
async fn test_query_tip() {
    let pool = connect_and_create_tables().await;
    let res = pool.query_tip().await.unwrap();
    assert!(res.is_none());

    let pool = connect_and_insert_blocks().await;
    let (block_number, block_hash) = pool.query_tip().await.unwrap().unwrap();
    assert_eq!(9, block_number);
    assert_eq!(
        "953761d56c03bfedf5e70dde0583470383184c41331f709df55d4acab5358640".to_string(),
        block_hash.to_string()
    );
}

#[tokio::test]
async fn test_get_canonical_block_hash() {
    let pool = connect_and_insert_blocks().await;
    let res = pool
        .get_canonical_block_hash(Context::new(), 0)
        .await
        .unwrap();
    assert_eq!(
        h256!("0x10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606"),
        res
    );
}
