use common::{anyhow::Result, DetailedCell, PaginationRequest, PaginationResponse, Range};
pub use xsql::{DBAdapter, DBDriver, DBInfo, TransactionInfo, XSQLPool, DB};

use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{bytes::Bytes, packed, H160, H256};

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
    pub fn new(adapter: Arc<T>, max_connections: u32, center_id: u16, machine_id: u16) -> Self {
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

    pub async fn sync_blocks(
        &self,
        start: BlockNumber,
        end: BlockNumber,
        batch_size: usize,
    ) -> Result<()> {
        self.inner.sync_blocks(start, end, batch_size).await
    }

    pub async fn get_scripts(
        &self,
        script_hashes: Vec<H160>,
        code_hash: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<packed::Script>> {
        self.inner
            .get_scripts(script_hashes, code_hash, args_len, args, pagination)
            .await
    }

    pub async fn get_live_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        self.inner
            .get_live_cells(
                out_point,
                lock_hashes,
                type_hashes,
                block_number,
                block_range,
                pagination,
            )
            .await
    }

    pub async fn get_transactions(
        &self,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionView>> {
        self.inner
            .get_transactions(tx_hashes, lock_hashes, type_hashes, block_range, pagination)
            .await
    }

    pub async fn get_transaction_info_by_hash(&self, tx_hash: H256) -> Result<TransactionInfo> {
        self.inner.get_transaction_info_by_hash(tx_hash).await
    }

    pub async fn get_spent_transaction_hash(
        &self,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>> {
        self.inner.get_spent_transaction_hash(out_point).await
    }

    pub async fn get_block_header(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView> {
        self.inner.get_block_header(block_hash, block_number).await
    }

    pub async fn register_addresses(&self, addresses: Vec<(H160, String)>) -> Result<Vec<H160>> {
        self.inner.register_addresses(addresses).await
    }

    pub async fn get_canonical_block_hash(&self, block_number: BlockNumber) -> Result<H256> {
        self.inner.get_canonical_block_hash(block_number).await
    }

    pub async fn get_script_by_partical_arg(
        &self,
        code_hash: H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>> {
        self.inner
            .get_scripts_by_partial_arg(code_hash, arg, offset_location)
            .await
    }

    pub fn get_db_info(&self) -> Result<DBInfo> {
        self.inner.get_db_info()
    }

    pub async fn get_spent_transaction_info(
        &self,
        _outpoint: packed::OutPoint,
    ) -> Result<TransactionInfo> {
        todo!()
    }

    pub async fn get_spent_transaction_view(
        &self,
        _outpoint: packed::OutPoint,
    ) -> Result<TransactionView> {
        todo!()
    }
}
