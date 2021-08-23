use common::anyhow::Result;
pub use xsql::{DBAdapter, DBDriver, DBInfo, DetailedCell, XSQLPool, DB};

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::H256;

use std::sync::Arc;

#[derive(Debug)]
pub struct MercuryStore<T> {
    pub inner: Arc<XSQLPool<T>>,
}

impl<T> Clone for MercuryStore<T> {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        MercuryStore { inner }
    }
}

impl<T: DBAdapter> MercuryStore<T> {
    pub fn new(adapter: T, max_connections: u32, center_id: u16, machine_id: u16) -> Self {
        let pool = XSQLPool::new(adapter, max_connections, center_id, machine_id);
        MercuryStore {
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

impl<T: DBAdapter> MercuryStore<T> {
    pub async fn append_block(&self, block: BlockView) -> Result<()> {
        self.inner.append_block(block).await
    }

    pub async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()> {
        self.inner.rollback_block(block_number, block_hash).await
    }

    pub async fn get_tip(&self) -> Result<Option<(BlockNumber, H256)>> {
        self.inner.get_tip().await
    }
}
