pub mod error;
mod fetch;
mod insert;
mod page;
mod remove;
mod snowflake;
mod sql;
mod synchronize;
pub mod table;
#[cfg(test)]
mod tests;

pub use db_protocol::{DBAdapter, DBDriver, DBInfo, TransactionInfo, DB};
pub use table::BsonBytes;

use crate::synchronize::{handle_out_point, sync_blocks_process};
use crate::{error::DBError, page::CursorPagePlugin, snowflake::Snowflake};

use common::{
    anyhow::Result, async_trait, utils::to_fixed_array, DetailedCell, PaginationRequest,
    PaginationResponse, Range,
};

use bson::spec::BinarySubtype;
use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{bytes::Bytes, packed, H160, H256};
use log::LevelFilter;
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::{
    core::db::DBPoolOptions, plugin::log::RbatisLogPlugin, rbatis::Rbatis, wrapper::Wrapper,
};
use tokio::sync::mpsc::unbounded_channel;

use std::sync::Arc;

const CHUNK_BLOCK_NUMBER: usize = 10_000;

lazy_static::lazy_static! {
    pub static ref SNOWFLAKE: Snowflake = Snowflake::default();
}

#[derive(Debug)]
pub struct XSQLPool<T> {
    adapter: T,
    inner: Arc<Rbatis>,
    config: DBPoolOptions,
}

#[async_trait]
impl<T: DBAdapter> DB for XSQLPool<T> {
    async fn append_block(&self, block: BlockView) -> Result<()> {
        let mut tx = self.transaction().await?;

        self.insert_block_table(&block, &mut tx).await?;
        self.insert_transaction_table(&block, &mut tx).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()> {
        let mut tx = self.transaction().await?;
        let block_hash = to_bson_bytes(&block_hash.0);

        self.remove_tx_and_cell(block_number, block_hash.clone(), &mut tx)
            .await?;
        self.remove_canonical_chain(block_number, block_hash, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    async fn get_live_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        self.query_live_cells(
            lock_hashes,
            type_hashes,
            block_number,
            block_range,
            pagination,
        )
        .await
    }

    async fn get_transactions(
        &self,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionView>> {
        let tx_hashes = tx_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let tx_tables = self
            .query_transactions(tx_hashes, lock_hashes, type_hashes, block_range, pagination)
            .await?;
        let tx_views = self.get_transaction_views(tx_tables.response).await?;
        Ok(fetch::to_pagination_response(
            tx_views,
            tx_tables.next_cursor,
            tx_tables.count.unwrap_or(0),
        ))
    }

    async fn get_block(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<BlockView> {
        match (block_hash, block_number) {
            (None, None) => self.get_tip_block().await,
            (None, Some(block_number)) => self.get_block_by_number(block_number).await,
            (Some(block_hash), None) => self.get_block_by_hash(block_hash).await,
            (Some(block_hash), Some(block_number)) => {
                let result = self.get_block_by_hash(block_hash).await;
                if let Ok(ref block_view) = result {
                    if block_view.number() != block_number {
                        return Err(DBError::MismatchBlockHash.into());
                    }
                }
                result
            }
        }
    }

    async fn get_block_header(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView> {
        match (block_hash, block_number) {
            (None, None) => self.get_tip_block_header().await,
            (None, Some(block_number)) => self.get_block_header_by_block_number(block_number).await,
            (Some(block_hash), None) => self.get_block_header_by_block_hash(block_hash).await,
            (Some(block_hash), Some(block_number)) => {
                let result = self.get_block_header_by_block_hash(block_hash).await;
                if let Ok(ref block_view) = result {
                    if block_view.number() != block_number {
                        return Err(DBError::MismatchBlockHash.into());
                    }
                }
                result
            }
        }
    }

    async fn get_scripts(
        &self,
        script_hashes: Vec<H160>,
        code_hashes: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<packed::Script>> {
        let script_hashes = script_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let code_hashes = code_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let args = args
            .into_iter()
            .map(|arg| to_bson_bytes(&arg))
            .collect::<Vec<_>>();

        self.query_scripts(script_hashes, code_hashes, args_len, args, pagination)
            .await
    }

    async fn get_tip(&self) -> Result<Option<(BlockNumber, H256)>> {
        self.query_tip().await
    }

    async fn get_spent_transaction_hash(
        &self,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>> {
        self.quert_spent_tx_hash(out_point).await
    }

    async fn get_transaction_info_by_hash(&self, tx_hash: H256) -> Result<TransactionInfo> {
        self.query_transaction_info(tx_hash).await
    }

    async fn sync_blocks(
        &self,
        start: BlockNumber,
        end: BlockNumber,
        batch_size: usize,
    ) -> Result<()> {
        assert!(start < end);
        let block_numbers = (start..=end).collect::<Vec<_>>();
        let (out_point_tx, out_point_rx) = unbounded_channel();
        let (number_tx, mut number_rx) = unbounded_channel();
        let rb_clone = Arc::clone(&self.inner);

        tokio::spawn(async move {
            handle_out_point(rb_clone, out_point_rx).await.unwrap();
        });

        for numbers in block_numbers.chunks(CHUNK_BLOCK_NUMBER).into_iter() {
            let blocks = self.adapter.pull_blocks(numbers.to_vec()).await?;
            let out_point_tx_clone = out_point_tx.clone();
            let number_tx_clone = number_tx.clone();
            let rb = Arc::clone(&self.inner);

            tokio::spawn(async move {
                sync_blocks_process::<T>(
                    rb,
                    blocks,
                    out_point_tx_clone,
                    number_tx_clone,
                    batch_size,
                )
                .await
                .unwrap();
            });
        }

        let mut max_sync_number = BlockNumber::MIN;
        while let Some(num) = number_rx.recv().await {
            max_sync_number = max_sync_number.max(num);

            if max_sync_number == end {
                out_point_tx.closed().await;
                number_tx.closed().await;
                SNOWFLAKE.clear_sequence();
                return Ok(());
            }
        }

        Ok(())
    }

    async fn get_registered_address(&self, lock_hash: H160) -> Result<Option<String>> {
        let lock_hash = to_bson_bytes(lock_hash.as_bytes());
        let res = self.query_registered_address(lock_hash).await?;
        Ok(res.map(|t| t.address))
    }

    async fn register_addresses(&self, addresses: Vec<(H160, String)>) -> Result<Vec<H160>> {
        let mut tx = self.transaction().await?;
        let addresses = addresses
            .into_iter()
            .map(|(lock_hash, address)| (to_bson_bytes(lock_hash.as_bytes()), address))
            .collect::<Vec<_>>();
        let res = self
            .insert_registered_address_table(addresses, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(res
            .iter()
            .map(|hash| H160(to_fixed_array::<20>(&hash.bytes)))
            .collect())
    }

    fn get_db_info(&self) -> Result<DBInfo> {
        let info = SNOWFLAKE.get_info();

        Ok(DBInfo {
            version: clap::crate_version!().to_string(),
            db: DBDriver::PostgreSQL,
            conn_size: self.config.max_connections,
            center_id: info.0,
            machine_id: info.1,
        })
    }
}

impl<T: DBAdapter> XSQLPool<T> {
    pub fn new(adapter: T, max_connections: u32, center_id: u16, machine_id: u16) -> Self {
        let config = DBPoolOptions {
            max_connections,
            ..Default::default()
        };

        let mut inner = Rbatis::new();
        inner.set_page_plugin(CursorPagePlugin::default());

        SNOWFLAKE.set_info(center_id, machine_id);

        XSQLPool {
            adapter,
            inner: Arc::new(inner),
            config,
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
            .link_opt(
                &build_url(db_driver.into(), db_name, host, port, user, password),
                &self.config,
            )
            .await
            .unwrap();
        Ok(())
    }

    pub async fn transaction(&self) -> Result<RBatisTxExecutor<'_>> {
        let tx = self.inner.acquire_begin().await?;
        Ok(tx)
    }

    async fn acquire(&self) -> Result<RBatisConnExecutor<'_>> {
        let conn = self.inner.acquire().await?;
        Ok(conn)
    }

    fn wrapper(&self) -> Wrapper {
        self.inner.new_wrapper()
    }
}

pub fn generate_id(block_number: BlockNumber) -> i64 {
    let number = block_number as i64;
    SNOWFLAKE.generate(number)
}

fn build_url(
    db_type: &str,
    db_name: &str,
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> String {
    if db_type == db_protocol::SQLITE {
        return db_type.to_string() + db_name;
    }

    db_type.to_string()
        + user
        + ":"
        + password
        + "@"
        + host
        + ":"
        + port.to_string().as_str()
        + "/"
        + db_name
}

pub fn log_plugin(level_filter: LevelFilter) -> RbatisLogPlugin {
    RbatisLogPlugin { level_filter }
}

pub fn to_bson_bytes(input: &[u8]) -> BsonBytes {
    BsonBytes {
        subtype: BinarySubtype::Generic,
        bytes: input.to_vec(),
    }
}

pub fn empty_bson_bytes() -> BsonBytes {
    BsonBytes {
        subtype: BinarySubtype::Generic,
        bytes: vec![],
    }
}
