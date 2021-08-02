use common::{anyhow::Result, Order, Range};

use async_trait::async_trait;
use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{bytes::Bytes, packed, H160, H256, U256};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait DB {
    ///
    async fn append_block(&self, block: BlockView) -> Result<()>;

    ///
    async fn rollback_block(&self, block_hash: H256) -> Result<()>;

    ///
    async fn get_live_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<Vec<DetailedCell>>;

    ///
    async fn get_transactions(
        &self,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Range,
        order: Order,
        limit: usize,
        skip: usize,
    ) -> Result<Vec<TransactionView>>;

    ///
    async fn get_block(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView>;

    ///
    async fn get_block_header(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<BlockView>;

    ///
    async fn get_scripts(
        &self,
        script_hashes: Vec<H160>,
        code_hash: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<String>,
    ) -> Result<Vec<packed::Script>>;

    ///
    fn get_db_info(&self) -> Result<DBInfo>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub enum DBKind {
    PostgreSQL,
}

#[derive(Clone, Debug)]
pub struct DetailedCell {
    pub epoch_number: U256,
    pub block_number: BlockNumber,
    pub block_hash: H256,
    pub out_point: packed::OutPoint,
    pub cell_output: packed::CellOutput,
    pub cell_data: Bytes,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct DBInfo {
    pub db: DBKind,
}
