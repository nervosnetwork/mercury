use super::*;

#[tokio::test]
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

#[tokio::test]
async fn test_get_consumed_cell() {
    connect_and_insert_blocks().await;
    let mut conn = TEST_POOL.pool.acquire().await.unwrap();
    let tx_hashes = vec![to_bson_bytes(
        &h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37").0,
    )];

    let res = sql::fetch_consume_cell_by_txs_sqlite(&mut conn, tx_hashes)
        .await
        .unwrap();
    println!("{:?}", res);
}
