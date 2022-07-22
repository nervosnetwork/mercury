mod sync_test;

use core_storage::{relational::RelationalStorage, DBDriver};

const MEMORY_DB: &str = ":memory:";

async fn connect_sqlite() -> RelationalStorage {
    let mut pool = RelationalStorage::new(0, 0, 100, 0, 60, 1800, 30, log::LevelFilter::Info);
    pool.connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
        .await
        .unwrap();
    pool
}

async fn connect_and_create_tables() -> RelationalStorage {
    let pool = connect_sqlite().await;
    let tx = pool.sqlx_pool.transaction().await.unwrap();
    xsql_test::create_tables(tx).await.unwrap();
    pool
}
