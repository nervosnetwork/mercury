use common::{
    anyhow::Result, async_trait, DetailedCell, PaginationRequest, PaginationResponse, Range,
};

use ckb_types::core::{BlockNumber, BlockView, HeaderView, RationalU256, TransactionView};
use ckb_types::{bytes::Bytes, packed, H160, H256};

use serde::{Deserialize, Serialize};

pub const MYSQL: &str = "mysql://";
pub const PGSQL: &str = "postgres://";
pub const SQLITE: &str = "sqlite://";

#[async_trait]
pub trait DB {
    /// Append the given block to the database.
    async fn append_block(&self, block: BlockView) -> Result<()>;

    /// Rollback a block by block hash and block number from the database.
    async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()>;

    /// Get live cells from the database according to the given arguments.
    async fn get_live_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get transactions from the database according to the given arguments.
    async fn get_transactions(
        &self,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionView>>;

    /// Get the block from the database.
    /// There are four situations for the combination of `block_hash` and `block_number`:
    /// 1. `block_hash` and `block_number` are both `Some`. Firstly get block by hash and
    /// check the block number is right.
    /// 2. 'block_hash' is `Some` and 'block_number' is 'None'. Get block by block hash.
    /// 3. 'block_hash' is `None` and 'block_number' is 'Some'. Get block by block number.
    /// 4. 'block_hash' and `block_number` are both None. Get tip block.
    async fn get_block(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<BlockView>;

    /// Get the block header from the database.
    /// There are four situations for the combination of `block_hash` and `block_number`:
    /// 1. `block_hash` and `block_number` are both `Some`. Firstly get block header by hash
    /// and check the block number is right.
    /// 2. 'block_hash' is `Some` and 'block_number' is 'None'. Get block header by block hash.
    /// 3. 'block_hash' is `None` and 'block_number' is 'Some'. Get block header by block number.
    /// 4. 'block_hash' and `block_number` are both None. Get tip block header.
    async fn get_block_header(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView>;

    /// Get scripts from the database according to the given arguments.
    async fn get_scripts(
        &self,
        script_hashes: Vec<H160>,
        code_hash: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<packed::Script>>;

    async fn get_tip(&self) -> Result<Option<(BlockNumber, H256)>>;

    /// Synchronize blocks by block number from start to end.
    async fn sync_blocks(
        &self,
        start: BlockNumber,
        end: BlockNumber,
        batch_size: usize,
    ) -> Result<()>;

    ///
    async fn get_epoch_number_by_transaction(&self, tx_hash: H256) -> Result<RationalU256>;

    ///
    async fn get_block_number_by_transaction(&self, tx_hash: H256) -> Result<BlockNumber>;
    
    /// Get lock hash by registered address
    async fn get_registered_address(&self, lock_hash: H160) -> Result<String>;

    /// Register address
    async fn register_address(&self, lock_hash: H160, address: String) -> Result<()>;

    /// Get the database information.
    fn get_db_info(&self) -> Result<DBInfo>;
}

#[async_trait]
pub trait DBAdapter: Sync + Send + 'static {
    /// Pull blocks by block number when synchronizing.
    async fn pull_blocks(&self, block_numbers: Vec<BlockNumber>) -> Result<Vec<BlockView>>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub enum DBDriver {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl Default for DBDriver {
    fn default() -> Self {
        DBDriver::PostgreSQL
    }
}

impl DBDriver {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "postgres" => DBDriver::PostgreSQL,
            "mysql" => DBDriver::MySQL,
            "sqlite" => DBDriver::SQLite,
            _ => panic!("Invalid DB driver type"),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash)]
pub struct DBInfo {
    pub version: String,
    pub db: DBDriver,
    pub conn_size: u32,
    pub center_id: i64,
    pub machine_id: i64,
}

#[allow(clippy::from_over_into)]
impl Into<&str> for DBDriver {
    fn into(self) -> &'static str {
        match self {
            DBDriver::PostgreSQL => PGSQL,
            DBDriver::MySQL => MYSQL,
            DBDriver::SQLite => SQLITE,
        }
    }
}
