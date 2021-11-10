use super::*;

#[ignore]
#[tokio::test]
async fn test_is_not_live_cell() {
    let pool = connect_pg_pool().await;
    let mut conn = pool.acquire().await.unwrap();
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
    let pool = connect_pg_pool().await;
    let mut conn = pool.acquire().await.unwrap();
    let tx_hash =
        hex::decode("5c00f96dda085b79c41abb8bd29c3a00fef6ddd2b25d20e647886e75c604a5fa").unwrap();
    let res = sql::is_live_cell(&mut conn, &to_rb_bytes(&tx_hash), &0)
        .await
        .unwrap();
    assert!(res.is_some());
}
