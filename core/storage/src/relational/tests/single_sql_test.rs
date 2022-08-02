use super::*;

use ckb_types::H256;
use std::str::FromStr;

#[tokio::test]
async fn test_fetch_distinct_tx_hashes_count_by_range_() {
    let pool = connect_and_insert_blocks().await;

    let res = pool
        .query_distinct_tx_hashes_count(&[], &[], &Some(Range::new(0, 10)), false)
        .await
        .unwrap();
    assert_eq!(2, res);

    let res = pool
        .query_distinct_tx_hashes_count(&[], &[], &Some(Range::new(1, 10)), false)
        .await
        .unwrap();
    assert_eq!(0, res);

    let res = pool
        .query_distinct_tx_hashes_count(&[], &[], &None, false)
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_fetch_distinct_tx_hashes_count_by_lock_hash() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash =
        H256::from_str("ba93972fbe398074f4e0bc538d7e36e61a8b140585b52deb4d2890e8d9d320f0").unwrap();

    let res = pool
        .query_distinct_tx_hashes_count(&[lock_hash.clone()], &[], &Some(Range::new(0, 10)), false)
        .await
        .unwrap();
    assert_eq!(1, res);

    let res = pool
        .query_distinct_tx_hashes_count(&[lock_hash.clone()], &[], &Some(Range::new(1, 10)), false)
        .await
        .unwrap();
    assert_eq!(0, res);

    let res = pool
        .query_distinct_tx_hashes_count(&[lock_hash], &[], &None, false)
        .await
        .unwrap();
    assert_eq!(1, res);
}
