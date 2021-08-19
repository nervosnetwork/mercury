pub mod error;
mod fetch;
mod insert;
mod remove;
mod snowflake;
mod sql;
mod synchronize;
pub mod table;

pub use db_protocol::{DBAdapter, DBDriver, DBInfo, DetailedCell, DB};
use error::DBError;
use snowflake::Snowflake;
use synchronize::{handle_out_point, sync_blocks_process};
pub use table::BsonBytes;

use common::{anyhow::Result, async_trait, PaginationRequest, PaginationResponse, Range};

use bson::spec::BinarySubtype;
use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{packed, H160, H256};
use log::LevelFilter;
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::plugin::log::{LogPlugin, RbatisLogPlugin};
use rbatis::{core::db::DBPoolOptions, rbatis::Rbatis, wrapper::Wrapper};
use tokio::sync::mpsc::unbounded_channel;

const CHUNK_BLOCK_NUMBER: usize = 10_000;

lazy_static::lazy_static! {
    pub static ref SNOWFLAKE: Snowflake = Snowflake::default();
}

#[derive(Debug)]
pub struct XSQLPool<T> {
    adapter: T,
    inner: Rbatis,
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
        _lock_hashes: Vec<H256>,
        _type_hashes: Vec<H256>,
        _block_number: Option<BlockNumber>,
        _block_range: Option<Range>,
        _pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        todo!()
    }

    async fn get_transactions(
        &self,
        _tx_hashes: Vec<H256>,
        _lock_hashes: Vec<H256>,
        _type_hashes: Vec<H256>,
        _block_range: Option<Range>,
        _pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionView>> {
        todo!()
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
        _script_hashes: Vec<H160>,
        _code_hash: Vec<H256>,
        _args_len: Option<usize>,
        _args: Vec<String>,
        _pagination: PaginationRequest,
    ) -> Result<PaginationResponse<packed::Script>> {
        todo!()
    }

    async fn sync_blocks(&'static self, start: BlockNumber, end: BlockNumber) -> Result<()> {
        assert!(start < end);
        let block_numbers = (start..=end).collect::<Vec<_>>();
        let (out_point_tx, out_point_rx) = unbounded_channel();
        let (number_tx, mut number_rx) = unbounded_channel();
        let conn = self.acquire().await?;

        tokio::spawn(async move {
            handle_out_point(conn, out_point_rx).await.unwrap();
        });

        for numbers in block_numbers.chunks(CHUNK_BLOCK_NUMBER).into_iter() {
            let blocks = self.adapter.pull_blocks(numbers.to_vec()).await?;
            let exec_tx = self.transaction().await?;
            let out_point_tx_clone = out_point_tx.clone();
            let number_tx_clone = number_tx.clone();

            tokio::spawn(async move {
                sync_blocks_process::<T>(exec_tx, blocks, out_point_tx_clone, number_tx_clone)
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

    fn get_db_info(&self) -> Result<DBInfo> {
        let info = SNOWFLAKE.get_info();

        Ok(DBInfo {
            version: clap::crate_version!(),
            db: DBDriver::PostgreSQL,
            conn_size: self.config.max_connections,
            center_id: info.0,
            machine_id: info.1,
        })
    }
}

impl<T: DBAdapter> XSQLPool<T> {
    pub async fn new(
        adapter: T,
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        max_connections: u32,
        center_id: u16,
        machine_id: u16,
    ) -> Self {
        let config = DBPoolOptions {
            max_connections,
            ..Default::default()
        };

        let inner = Rbatis::new();
        inner
            .link_opt(
                &build_url(db_driver.into(), db_name, host, port, user, password),
                &config,
            )
            .await
            .unwrap();

        SNOWFLAKE.set_info(center_id, machine_id);

        XSQLPool {
            adapter,
            inner,
            config,
        }
    }

    async fn transaction(&self) -> Result<RBatisTxExecutor<'_>> {
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

    pub fn set_log_plugin(&mut self, plugin: impl LogPlugin + 'static) {
        self.inner.set_log_plugin(plugin)
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
