use super::*;

#[tokio::test]
async fn test_insert() {
    connect_and_insert_blocks().await;
}

#[tokio::test]
async fn test_remove_all() {
    connect_and_insert_blocks().await;
    let mut tx = TEST_POOL.pool.transaction().await.unwrap();
    xsql_test::delete_all_data(&mut tx).await.unwrap();
}

#[tokio::test]
async fn test_register_addresses() {
    connect_sqlite().await;
    let mut tx = TEST_POOL.pool.transaction().await.unwrap();
    xsql_test::create_tables(&mut tx).await.unwrap();

    let lock_hash = h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64");
    let address = String::from("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v");
    let addresses = vec![(lock_hash.clone(), address.clone())];
    let res = TEST_POOL
        .register_addresses(addresses.clone())
        .await
        .unwrap();
    assert_eq!(res[0], lock_hash);
    let res = TEST_POOL.get_registered_address(lock_hash).await.unwrap();
    assert_eq!(res, Some(address));
}

#[tokio::test]
async fn test_get_db_info() {
    connect_sqlite().await;
    let res = TEST_POOL.get_db_info().unwrap();
    assert_eq!(res.version, clap::crate_version!().to_string());
    assert_eq!(res.db, DBDriver::PostgreSQL);
    assert_eq!(res.center_id, 0);
    assert_eq!(res.machine_id, 0);
    assert_eq!(res.conn_size, 100);
}
