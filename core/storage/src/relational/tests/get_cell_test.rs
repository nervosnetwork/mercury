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
                cursor: Some(ckb_types::bytes::Bytes::from(
                    [127, 255, 255, 255, 255, 255, 255, 254].to_vec(),
                )),
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
                cursor: Some(ckb_types::bytes::Bytes::from(
                    [127, 255, 255, 255, 255, 255, 255, 254].to_vec(),
                )),
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

#[ignore]
#[tokio::test]
async fn test_is_not_live_cell() {
    let pool = connect_pg_pool().await;
    let mut conn = pool.pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("e2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c").unwrap();
    let res = sql::is_live_cell(&mut conn, &to_rb_bytes(&tx_hash), &5)
        .await
        .unwrap();
    assert!(res.is_none());
}

#[ignore]
#[tokio::test]
async fn test_is_live_cell() {
    let rdb = connect_pg_pool().await;
    let mut conn = rdb.pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("5c00f96dda085b79c41abb8bd29c3a00fef6ddd2b25d20e647886e75c604a5fa").unwrap();
    let res = sql::is_live_cell(&mut conn, &to_rb_bytes(&tx_hash), &0)
        .await
        .unwrap();
    assert!(res.is_some());
}
