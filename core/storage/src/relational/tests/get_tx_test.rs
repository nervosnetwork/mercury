use super::*;

#[tokio::test]
async fn test_get_txs() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash: H256 = caculate_lock_hash(
        "709f3fda12f561cfacf92273c57a98fede188a3f1a59b1f888d113f9cce08649",
        "b73961e46d9eb118d3de1d1e8f30b3af7bbf3160",
        ScriptHashType::Data,
    );

    let txs_from_db = pool
        .get_transactions(
            None,
            vec![lock_hash],
            vec![],
            Some(Range::new(0, 9)),
            false,
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response;
    let tx_hashes_from_db: Vec<H256> = txs_from_db
        .iter()
        .map(|tx| tx.transaction_with_status.transaction.clone().unwrap().hash)
        .collect();

    assert_eq!(2, txs_from_db.len());
    assert_eq!(
        "8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
        &tx_hashes_from_db[0].to_string()
    );
    assert_eq!(
        "f8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
        &tx_hashes_from_db[1].to_string()
    );
}

#[tokio::test]
async fn test_get_txs_by_block_range() {
    let pool = connect_and_insert_blocks().await;
    let txs_from_db = pool
        .get_transactions(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            false,
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response;
    let tx_hashes_from_db: Vec<H256> = txs_from_db
        .iter()
        .map(|tx| tx.transaction_with_status.transaction.clone().unwrap().hash)
        .collect();

    let mut txs_from_json: Vec<ckb_jsonrpc_types::TransactionView> = vec![];
    for i in 0..10 {
        let block: ckb_jsonrpc_types::BlockView = read_block_view(i, BLOCK_DIR.to_string());
        let mut txs = block.transactions;
        txs_from_json.append(&mut txs);
    }
    let tx_hashes_from_json: Vec<H256> = txs_from_json.iter().map(|tx| tx.hash.clone()).collect();

    assert_eq!(tx_hashes_from_db, tx_hashes_from_json);
}

#[tokio::test]
async fn test_get_spent_transaction_hash() {
    let pool = connect_and_insert_blocks().await;
    let block: BlockView = read_block_view(0, BLOCK_DIR.to_string()).into();
    let tx = &block.transaction(0).unwrap();
    let outpoint = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx.hash().unpack(),
        index: 0u32.into(),
    };
    let res = pool
        .get_spent_transaction_hash(outpoint.into())
        .await
        .unwrap();
    assert_eq!(res, None)
}

#[tokio::test]
async fn test_get_tx_timestamp() {
    let pool = connect_and_insert_blocks().await;
    let txs_from_db = pool
        .get_transactions(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            false,
            PaginationRequest::new(Some(0), Order::Asc, Some(20), None, true),
        )
        .await
        .unwrap()
        .response;
    let timestamps: Vec<u64> = txs_from_db.iter().map(|tx| tx.timestamp).collect();

    let mut timestamps_from_json: Vec<u64> = vec![];
    for i in 0..10 {
        let block: ckb_jsonrpc_types::BlockView = read_block_view(i, BLOCK_DIR.to_string());
        let txs = block.transactions;
        for _ in txs {
            let timestamp = block.header.inner.timestamp.into();
            timestamps_from_json.push(timestamp);
        }
    }

    assert_eq!(timestamps_from_json, timestamps);
}

#[tokio::test]
async fn test_get_simple_transaction_by_hash() {
    let pool = connect_and_insert_blocks_16().await;

    let simple_tx = pool
        .get_simple_transaction_by_hash(h256!(
            "0xa6789f42b0568b1872e5a5858f0c42148dd8d313f844252f5fe3dfe556958ba9"
        ))
        .await
        .unwrap();

    assert_eq!(
        "fb27201670e48f65b93b58c4cac7348c54554ad831ed5c1b386c9bd3c24fa911".to_string(),
        simple_tx.block_hash.to_string()
    );
    assert_eq!(0, simple_tx.tx_index);
    assert_eq!(12, simple_tx.block_number);
    println!("{:?}", simple_tx.epoch_number.to_string());
}

#[tokio::test]
async fn test_query_spent_tx_hash() {
    let pool = connect_and_insert_blocks_16().await;

    let tx_hash =
        h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37").pack();
    let out_point = packed::OutPoint::new(tx_hash, 1);
    let spent_tx = pool.query_spent_tx_hash(out_point).await.unwrap();
    assert!(spent_tx.is_none());

    let tx_hash =
        h256!("0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").pack();
    let out_point = packed::OutPoint::new(tx_hash, 5);
    let spent_tx = pool.query_spent_tx_hash(out_point).await.unwrap().unwrap();
    assert_eq!(
        spent_tx,
        h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37")
    );
}
