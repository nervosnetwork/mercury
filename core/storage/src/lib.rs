#![allow(clippy::mutable_key_type)]

pub mod error;
pub mod relational;

pub use relational::RelationalStorage;

use ckb_types::core::{BlockNumber, BlockView, HeaderView};
use ckb_types::{bytes::Bytes, packed, H160, H256};
use common::{async_trait, DetailedCell, PaginationRequest, PaginationResponse, Range, Result};
use core_rpc_types::indexer::Transaction;
pub use protocol::db::{DBDriver, DBInfo, SimpleBlock, SimpleTransaction, TransactionWrapper};

#[async_trait]
pub trait Storage {
    /// Append the given block to the database.
    async fn append_block(&self, block: BlockView) -> Result<()>;

    /// Rollback a block by block hash and block number from the database.
    async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()>;

    /// Get live cells from the database according to the given arguments.
    async fn get_live_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        lock_len_range: Option<Range>,
        type_len_range: Option<Range>,
        block_range: Option<Range>,
        capacity_range: Option<Range>,
        data_len_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get live cells from the database according to the given arguments.
    async fn get_historical_live_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: BlockNumber,
        out_point: Option<packed::OutPoint>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get cells from the database according to the given arguments.
    async fn get_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get transactions from the database according to the given arguments.
    async fn get_transactions(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        limit_cellbase: bool,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_transactions_by_hashes(
        &self,
        tx_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_transactions_by_scripts(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        limit_cellbase: bool,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

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
    ) -> Result<Vec<packed::Script>>;

    /// Get the tip number and block hash in database.
    async fn get_tip(&self) -> Result<Option<(BlockNumber, H256)>>;

    ///
    async fn get_simple_transaction_by_hash(&self, tx_hash: H256) -> Result<SimpleTransaction>;

    ///
    async fn get_spent_transaction_hash(&self, out_point: packed::OutPoint)
        -> Result<Option<H256>>;

    ///
    async fn get_canonical_block_hash(&self, block_number: BlockNumber) -> Result<H256>;

    ///
    async fn get_scripts_by_partial_arg(
        &self,
        code_hash: &H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>>;

    /// Get lock hash by registered address
    async fn get_registered_address(&self, lock_hash: H160) -> Result<Option<String>>;

    /// Register address
    async fn register_addresses(&self, addresses: Vec<(H160, String)>) -> Result<Vec<H160>>;

    /// Get the database information.
    fn get_db_info(&self) -> Result<DBInfo>;

    /// Get block info
    async fn get_simple_block(
        &self,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<SimpleBlock>;

    /// Get the cells for indexer API.
    async fn get_indexer_transactions(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<Transaction>>;

    /// Get the block count.
    async fn indexer_synced_count(&self) -> Result<u64>;

    /// Get the block count.
    async fn block_count(&self) -> Result<u64>;
}
