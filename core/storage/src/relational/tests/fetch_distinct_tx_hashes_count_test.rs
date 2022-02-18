use super::*;

use ckb_types::H256;
use std::str::FromStr;

#[tokio::test]
async fn test_fetch_distinct_tx_hashes_count_by_range() {
    let pool = connect_and_insert_blocks().await.pool;
    let mut conn = pool.acquire().await.unwrap();

    let res = sql::fetch_distinct_tx_hashes_count(&mut conn, &0, &10, &[], &[], &true).await;
    assert_eq!(2, res.unwrap());

    let res = sql::fetch_distinct_tx_hashes_count(&mut conn, &1, &10, &[], &[], &true).await;
    assert_eq!(0, res.unwrap());

    let res = sql::fetch_distinct_tx_hashes_count(&mut conn, &1, &10, &[], &[], &false).await;
    assert_eq!(2, res.unwrap());
}

#[tokio::test]
async fn test_fetch_distinct_tx_hashes_count_by_lock_hash() {
    let pool = connect_and_insert_blocks().await.pool;
    let mut conn = pool.acquire().await.unwrap();

    let lock_hash =
        H256::from_str("ba93972fbe398074f4e0bc538d7e36e61a8b140585b52deb4d2890e8d9d320f0").unwrap();

    let res = sql::fetch_distinct_tx_hashes_count(
        &mut conn,
        &0,
        &10,
        &[to_rb_bytes(&lock_hash.0)],
        &[],
        &true,
    )
    .await;
    assert_eq!(1, res.unwrap());

    let res = sql::fetch_distinct_tx_hashes_count(
        &mut conn,
        &1,
        &10,
        &[to_rb_bytes(&lock_hash.0)],
        &[],
        &true,
    )
    .await;
    assert_eq!(0, res.unwrap());

    let res = sql::fetch_distinct_tx_hashes_count(
        &mut conn,
        &1,
        &10,
        &[to_rb_bytes(&lock_hash.0)],
        &[],
        &false,
    )
    .await;
    assert_eq!(1, res.unwrap());
}
