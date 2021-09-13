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
            PaginationRequest::new(
                Some(Bytes::from(0i64.to_be_bytes().to_vec())),
                Order::Asc,
                Some(20),
                None,
                true,
            ),
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

#[ignore]
#[tokio::test]
async fn test_is_not_live_cell() {
    let pool = connect_pg_pool().await;
    let mut conn = pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("e2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c").unwrap();
    let res = sql::is_live_cell(&mut conn, to_bson_bytes(&tx_hash), 5)
        .await
        .unwrap();
    assert!(res.is_none());
}

#[ignore]
#[tokio::test]
async fn test_is_live_cell() {
    let pool = connect_pg_pool().await;
    let mut conn = pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("5c00f96dda085b79c41abb8bd29c3a00fef6ddd2b25d20e647886e75c604a5fa").unwrap();
    let res = sql::is_live_cell(&mut conn, to_bson_bytes(&tx_hash), 0)
        .await
        .unwrap();
    assert!(res.is_some());
}

#[tokio::test]
async fn test_fetch_consumed_cell() {
    let pool = connect_pg_pool().await;
    let mut conn = pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("119ab4e5a60e2f903c2ce4e58cdc5f6b8944ba3bcd7ffc08222e7d434914657d").unwrap();
    let res = sql::fetch_consume_cell_by_tx_hash(&mut conn, to_bson_bytes(&tx_hash))
        .await
        .unwrap();
    println!("{:?}", res);
}
