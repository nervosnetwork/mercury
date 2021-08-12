#![allow(dead_code)]
pub mod error;
mod fetch;
mod insert;
pub mod plugin;
mod sql;
mod table;

pub use db_protocol::{DBDriver, DBInfo, DetailedCell, DB};
use error::DBError;

use common::{anyhow::Result, async_trait, PaginationRequest, PaginationResponse, Range};

use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{packed, H160, H256};
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::plugin::{log::LogPlugin, snowflake::Snowflake};
use rbatis::{core::db::DBPoolOptions, rbatis::Rbatis, wrapper::Wrapper};

#[macro_export]
macro_rules! str {
    ($exp: expr) => {
        hex::encode($exp.as_slice())
    };
}

#[derive(Debug)]
pub struct XSQLPool {
    inner: Rbatis,
    config: DBPoolOptions,
    machine_id: i64,
    node_id: i64,
    id_generator: Snowflake,
}

#[async_trait]
impl DB for XSQLPool {
    async fn append_block(&self, block: BlockView) -> Result<()> {
        let mut tx = self.transaction().await?;
        self.insert_block_table(&block, &mut tx).await?;
        self.insert_transaction_table(&block, &mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn rollback_block(&self, _block_number: BlockNumber, _block_hash: H256) -> Result<()> {
        todo!()
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
        _block_hash: Option<H256>,
        _block_number: Option<BlockNumber>,
    ) -> Result<Vec<BlockView>> {
        todo!()
    }

    async fn get_block_header(
        &self,
        _block_hash: Option<H256>,
        _block_number: Option<BlockNumber>,
    ) -> Result<Vec<HeaderView>> {
        todo!()
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

    async fn sync_blocks(&self, _start: BlockNumber, _end: BlockNumber) -> Result<()> {
        todo!()
    }

    fn get_db_info(&self) -> Result<DBInfo> {
        Ok(DBInfo {
            version: clap::crate_version!(),
            db: DBDriver::PostgreSQL,
            conn_size: self.config.max_connections,
            machine_id: self.machine_id,
            node_id: self.node_id,
        })
    }
}

impl XSQLPool {
    pub async fn new(
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        max_connections: u32,
        machine_id: i64,
        node_id: i64,
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

        let mut id_generator = Snowflake::default();
        id_generator.datacenter_id(machine_id);
        id_generator.worker_id(node_id);

        XSQLPool {
            inner,
            config,
            machine_id,
            node_id,
            id_generator,
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

    pub fn generate_id(&self) -> i64 {
        self.id_generator.generate()
    }

    fn parse_get_block_request(
        &self,
        block_hash: Option<H256>,
        block_number: Option<u64>,
    ) -> Result<InnerBlockRequest> {
        if block_hash.is_none() && block_number.is_none() {
            return Err(DBError::InvalidGetBlockRequest.into());
        }

        let ret = if let Some(hash) = block_hash {
            InnerBlockRequest::ByHash(hash, block_number)
        } else {
            InnerBlockRequest::ByNumber(block_number.unwrap())
        };

        Ok(ret)
    }

    #[cfg(test)]
    pub async fn new_sqlite(path: &str) -> Self {
        let inner = Rbatis::new();
        let config = DBPoolOptions::default();
        inner.link_opt(path, &config).await.unwrap();

        let mut id_generator = Snowflake::default();
        id_generator.datacenter_id(1);
        id_generator.worker_id(1);

        XSQLPool {
            inner,
            config,
            machine_id: 1,
            node_id: 1,
            id_generator,
        }
    }
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

enum InnerBlockRequest {
    ByHash(H256, Option<u64>),
    ByNumber(u64),
}
