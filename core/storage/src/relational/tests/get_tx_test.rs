use super::*;

#[tokio::test]
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
        .map(From::from)
        .collect();

    let block: ckb_jsonrpc_types::BlockView = read_block_view(0, BLOCK_DIR.to_string());
    let txs_from_json: Vec<ckb_jsonrpc_types::TransactionView> = block.transactions;

    assert_eq!(txs_from_db[0].hash, txs_from_json[0].hash);
    assert_eq!(txs_from_db[1].hash, txs_from_json[1].hash);
}

#[tokio::test]
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
    let tx_hashes_from_db: Vec<H256> = txs_from_db.iter().map(|tx| tx.hash().unpack()).collect();

    let mut txs_from_json: Vec<ckb_jsonrpc_types::TransactionView> = vec![];
    for i in 1..10 {
        let block: ckb_jsonrpc_types::BlockView = read_block_view(i, BLOCK_DIR.to_string());
        let mut txs = block.transactions;
        txs_from_json.append(&mut txs);
    }
    let tx_hashes_from_json: Vec<H256> = txs_from_json.iter().map(|tx| tx.hash.clone()).collect();

    assert_eq!(tx_hashes_from_db, tx_hashes_from_json);
}

#[tokio::test]
async fn test_get_spent_transaction_hash() {
    connect_and_insert_blocks().await;
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
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
