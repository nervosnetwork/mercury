use super::*;

#[tokio::test]
async fn test_get_genesis_block_header() {
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
async fn test_get_genesis_block() {
    let pool = connect_and_insert_blocks().await;
    let res = pool.get_block(Context::new(), None, Some(0)).await.unwrap();
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
    assert_eq!(block.data(), res.data());
}

#[tokio::test]
async fn test_get_block_by_number() {
    let pool = connect_and_insert_blocks().await;
    let res = pool.get_block(Context::new(), None, Some(1)).await.unwrap();
    let block: BlockView = read_block_view(1, BLOCK_DIR.to_string()).into();
    assert_eq!(block.data(), res.data());
}

#[tokio::test]
async fn test_get_block_info() {
    let pool = connect_and_insert_blocks().await;
    let block_table = pool.query_block_by_number(0).await.unwrap();
    let tx_tables = pool
        .query_transactions_by_block_hash(&block_table.block_hash)
        .await
        .unwrap();
    let tx_hashes: Vec<H256> = tx_tables
        .iter()
        .map(|tx| bson_to_h256(&tx.tx_hash))
        .collect();

    let block_info = pool
        .get_simple_block(Context::new(), None, Some(0))
        .await
        .unwrap();
    assert_eq!(
        block_table.block_hash,
        to_rb_bytes(block_info.block_hash.as_bytes())
    );
    assert_eq!(tx_hashes, block_info.transactions);
}

#[tokio::test]
async fn test_get_genesis_block_hash() {
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
    let block_hash_from_table = bson_to_h256(&block_table.block_hash);
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
}

#[tokio::test]
async fn test_get_block_hash() {
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
    let block_hash_from_table = bson_to_h256(&block_table.block_hash);
    println!(
        "hash in block table: {:?}",
        block_hash_from_table.to_string()
    );

    assert_eq!(block_hash_from_json, block_hash_from_table);

    // from built block view
    let res = pool.get_block(Context::new(), None, Some(1)).await.unwrap();
    let block_built_hash: H256 = res.hash().unpack();
    println!("block hash is {:?}", block_built_hash.to_string());

    assert_eq!(block_hash_from_json, block_built_hash);
}
