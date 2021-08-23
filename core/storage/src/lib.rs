use common::anyhow::Result;
pub use xsql::{DBAdapter, DBDriver, DBInfo, DetailedCell, XSQLPool, DB};

use std::sync::Arc;

#[derive(Debug)]
pub struct MemoryStore<T> {
    inner: Arc<XSQLPool<T>>,
}

impl<T> Clone for MemoryStore<T> {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        MemoryStore { inner }
    }
}

impl<T: DBAdapter> MemoryStore<T> {
    pub fn new(adapter: T, max_connections: u32, center_id: u16, machine_id: u16) -> Self {
        let pool = XSQLPool::new(adapter, max_connections, center_id, machine_id);
        MemoryStore {
            inner: Arc::new(pool),
        }
    }

    pub async fn connect(
        &self,
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.inner
            .connect(db_driver, db_name, host, port, user, password)
            .await?;
        Ok(())
    }
}
