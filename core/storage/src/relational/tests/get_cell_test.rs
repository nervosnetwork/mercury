use super::*;

use std::str::FromStr;

#[tokio::test]
async fn test_get_cells_pagination_return_count() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash =
        H256::from_str("ba93972fbe398074f4e0bc538d7e36e61a8b140585b52deb4d2890e8d9d320f0").unwrap();

    let cells = pool
        .get_cells(
            Context::new(),
            None,
            vec![lock_hash.clone()],
            vec![],
            None,
            PaginationRequest {
                cursor: Some(u64::MAX.into()),
                order: Order::Desc,
                limit: Some(1),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(None, cells.count);

    let cells = pool
        .get_cells(
            Context::new(),
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(Some(12), cells.count);
    assert_eq!(2, cells.response.len());
}

#[tokio::test]
async fn test_is_not_live_cell() {
    let pool = connect_and_insert_blocks().await;
    let mut conn = pool.pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("e2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c").unwrap();
    let res = sql::is_live_cell(&mut conn, &to_rb_bytes(&tx_hash), &5)
        .await
        .unwrap();
    assert!(res.is_none());
}

#[tokio::test]
async fn test_is_live_cell() {
    let pool = connect_and_insert_blocks().await;
    let mut conn = pool.pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").unwrap();
    let res = sql::is_live_cell(&mut conn, &to_rb_bytes(&tx_hash), &0)
        .await
        .unwrap();
    assert!(res.is_some());
}

#[tokio::test]
async fn test_to_rb_bytes() {
    let tx_hash = hex::decode("63000000000000000000000000000000").unwrap();
    let ret_rbatis_bytes = to_rb_bytes(&tx_hash);
    let ret_bytes = Bytes::from(tx_hash);
    assert_eq!(ret_rbatis_bytes.len(), ret_bytes.len());
}
