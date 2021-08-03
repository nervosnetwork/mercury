#![allow(dead_code)]

pub use db_protocol::{DBInfo, DBKind, DetailedCell, DB};

use common::{anyhow::Result, async_trait, Pagination, Range};

use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{packed, H160, H256};
use clap::crate_version;
use sqlx::{postgres::PgConnectOptions, PgPool, Postgres, Transaction};

#[derive(Clone, Debug)]
pub struct PostgreSQLPool {
    inner: PgPool,
}

#[async_trait]
impl DB for PostgreSQLPool {
    async fn append_block(&self, _block: BlockView) -> Result<()> {
        todo!()
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
        _pagination: Pagination,
    ) -> Result<Vec<DetailedCell>> {
        todo!()
    }

    async fn get_transactions(
        &self,
        _tx_hashes: Vec<H256>,
        _lock_hashes: Vec<H256>,
        _type_hashes: Vec<H256>,
        _block_range: Option<Range>,
        _pagination: Pagination,
    ) -> Result<Vec<TransactionView>> {
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
        _pagination: Pagination,
    ) -> Result<Vec<packed::Script>> {
        todo!()
    }

    fn get_db_info(&self) -> Result<DBInfo> {
        Ok(DBInfo {
            version: crate_version!(),
            db: DBKind::PostgreSQL,
            conn_size: self.inner.size(),
        })
    }
}

impl PostgreSQLPool {
    pub async fn new(host: &str, port: u16, user: &str, password: &str) -> Self {
        let pg_option = PgConnectOptions::new()
            .host(host)
            .port(port)
            .username(user)
            .password(password);
        let inner = PgPool::connect_with(pg_option).await.unwrap();

        PostgreSQLPool { inner }
    }

    async fn transaction<'c>(&self) -> Result<Transaction<'c, Postgres>> {
        let ret = self.inner.begin().await?;
        Ok(ret)
    }
}
