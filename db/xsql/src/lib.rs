#![allow(dead_code)]
mod insert;
pub mod plugin;
mod table;

pub use db_protocol::{DBInfo, DBKind, DetailedCell, DB};

use common::{anyhow::Result, async_trait, PaginationRequest, PaginationResponse, Range};

use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{packed, H160, H256};
use rbatis::core::db::DBPoolOptions;
use rbatis::plugin::log::LogPlugin;
use rbatis::{executor::RBatisTxExecutor, rbatis::Rbatis};

const PG_PREFIX: &str = "postgres://";

#[derive(Debug)]
pub struct XSQLPool {
    inner: Rbatis,
    config: DBPoolOptions,
}

#[async_trait]
impl DB for XSQLPool {
    async fn append_block(&self, block: BlockView) -> Result<()> {
        let mut tx = self.transaction().await?;
        self.insert_block_table(&block, &mut tx).await?;
        self.insert_transaction_table(&block, &mut tx).await?;

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
    ) -> Result<HeaderView> {
        todo!()
    }

    async fn get_block_header(
        &self,
        _block_hash: Option<H256>,
        _block_number: Option<BlockNumber>,
    ) -> Result<BlockView> {
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

    fn get_db_info(&self) -> Result<DBInfo> {
        Ok(DBInfo {
            version: clap::crate_version!(),
            db: DBKind::PostgreSQL,
            conn_size: self.config.max_connections,
        })
    }
}

impl XSQLPool {
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        max_connections: u32,
    ) -> Self {
        let config = DBPoolOptions {
            max_connections,
            ..Default::default()
        };

        let inner = Rbatis::new();
        inner
            .link_opt(&build_url(host, port, user, password), &config)
            .await
            .unwrap();

        XSQLPool { inner, config }
    }

    async fn transaction(&self) -> Result<RBatisTxExecutor<'_>> {
        let tx = self.inner.acquire_begin().await?;
        Ok(tx)
    }

    pub fn set_log_plugin(&mut self, plugin: impl LogPlugin + 'static) {
        self.inner.set_log_plugin(plugin)
    }

    #[cfg(test)]
    pub async fn new_sqlite(path: &str) -> Self {
        let inner = Rbatis::new();
        let config = DBPoolOptions::default();
        inner.link_opt(path, &config).await.unwrap();

        XSQLPool { inner, config }
    }
}

fn build_url(host: &str, port: u16, user: &str, password: &str) -> String {
    PG_PREFIX.to_owned() + user + ":" + password + "@" + host + ":" + port.to_string().as_str()
}
