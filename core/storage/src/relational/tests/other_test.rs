use super::*;

#[tokio::test]
async fn test_insert() {
    let _pool = connect_and_insert_blocks().await;
}

#[tokio::test]
async fn test_remove_all() {
    let pool = connect_and_insert_blocks().await;
    let mut tx = pool.pool.transaction().await.unwrap();
    xsql_test::delete_all_data(&mut tx).await.unwrap();
}

#[tokio::test]
async fn test_register_addresses() {
    let pool = connect_sqlite().await;
    let mut tx = pool.pool.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let lock_hash = h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64");
    let address = String::from("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v");
    let addresses = vec![(lock_hash.clone(), address.clone())];
    let res = pool
        .register_addresses(Context::new(), addresses.clone())
        .await
        .unwrap();
    assert_eq!(res[0], lock_hash);
    let res = pool
        .get_registered_address(Context::new(), lock_hash)
        .await
        .unwrap();
    assert_eq!(res, Some(address));
}

#[tokio::test]
async fn test_get_db_info() {
    let pool = connect_sqlite().await;
    let res = pool.get_db_info(Context::new()).unwrap();
    assert_eq!(res.version, clap::crate_version!().to_string());
    assert_eq!(res.db, DBDriver::PostgreSQL);
    assert_eq!(res.center_id, 0);
    assert_eq!(res.machine_id, 0);
    assert_eq!(res.conn_size, 100);
}

#[ignore]
#[tokio::test]
async fn test_get_tx_hash() {
    let rdb = connect_pg_pool().await;
    let mut tx = rdb.pool.transaction().await.unwrap();
    let block_hash =
        hex::decode("bc5969d7829ea32aca5784a9eb94cf219f084d2451706bec378f08e23c417ce3").unwrap();
    let res = sql::get_tx_hashes_by_block_hash(&mut tx, &to_rb_bytes(&block_hash))
        .await
        .unwrap();
    println!("{:?}", res);
}

#[tokio::test]
async fn test_decode_cursor() {
    let cursor = Bytes::from([127, 255, 255, 255, 255, 255, 255, 254].to_vec());
    let cursor = i64::from_be_bytes(common::utils::to_fixed_array(&cursor[0..8]));
    assert_eq!(cursor, i64::MAX - 1);
}

#[tokio::test]
async fn test_encode_cursor() {
    assert_eq!(
        [127, 255, 255, 255, 255, 255, 255, 254].to_vec(),
        (i64::MAX - 1).to_be_bytes().to_vec()
    );
}
