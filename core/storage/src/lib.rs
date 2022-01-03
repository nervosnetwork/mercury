#![allow(clippy::mutable_key_type)]

pub mod error;
pub mod kvdb;
pub mod relational;

pub use protocol::db::{DBDriver, DBInfo, SimpleBlock, SimpleTransaction, TransactionWrapper};
pub use relational::RelationalStorage;

use relational::table::IndexerCellTable;

use common::{
    async_trait, Context, DetailedCell, PaginationRequest, PaginationResponse, Range, Result,
};

use ckb_types::core::{BlockNumber, BlockView, HeaderView};
use ckb_types::{bytes::Bytes, packed, H160, H256};

#[async_trait]
pub trait Storage {
    /// Append the given block to the database.
    async fn append_block(&self, ctx: Context, block: BlockView) -> Result<()>;

    /// Rollback a block by block hash and block number from the database.
    async fn rollback_block(
        &self,
        ctx: Context,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<()>;

    /// Get live cells from the database according to the given arguments.
    async fn get_live_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get live cells from the database according to the given arguments.
    async fn get_historical_live_cells(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: BlockNumber,
        out_point: Option<packed::OutPoint>,
    ) -> Result<Vec<DetailedCell>>;

    /// Get cells from the database according to the given arguments.
    async fn get_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    /// Get transactions from the database according to the given arguments.
    async fn get_transactions(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_transactions_by_hashes(
        &self,
        ctx: Context,
        tx_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_transactions_by_scripts(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
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
        ctx: Context,
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
        ctx: Context,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView>;

    /// Get scripts from the database according to the given arguments.
    async fn get_scripts(
        &self,
        ctx: Context,
        script_hashes: Vec<H160>,
        code_hash: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
    ) -> Result<Vec<packed::Script>>;

    /// Get the tip number and block hash in database.
    async fn get_tip(&self, ctx: Context) -> Result<Option<(BlockNumber, H256)>>;

    ///
    async fn get_simple_transaction_by_hash(
        &self,
        ctx: Context,
        tx_hash: H256,
    ) -> Result<SimpleTransaction>;

    ///
    async fn get_spent_transaction_hash(
        &self,
        ctx: Context,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>>;

    ///
    async fn get_canonical_block_hash(
        &self,
        ctx: Context,
        block_number: BlockNumber,
    ) -> Result<H256>;

    ///
    async fn get_scripts_by_partial_arg(
        &self,
        ctx: Context,
        code_hash: H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>>;

    /// Get lock hash by registered address
    async fn get_registered_address(&self, ctx: Context, lock_hash: H160)
        -> Result<Option<String>>;

    /// Register address
    async fn register_addresses(
        &self,
        ctx: Context,
        addresses: Vec<(H160, String)>,
    ) -> Result<Vec<H160>>;

    /// Get the database information.
    fn get_db_info(&self, ctx: Context) -> Result<DBInfo>;

    /// Get block info
    async fn get_simple_block(
        &self,
        ctx: Context,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<SimpleBlock>;

    /// Get the cells for indexer API.
    async fn get_indexer_transactions(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<IndexerCellTable>>;

    /// Get the block count.
    async fn indexer_synced_count(&self) -> Result<u64>;

    /// Get the block count.
    async fn block_count(&self, ctx: Context) -> Result<u64>;
}

#[async_trait]
pub trait ExtensionStorage {
    async fn get_live_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    async fn get_historical_live_cells(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: BlockNumber,
    ) -> Result<Vec<DetailedCell>>;

    async fn get_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    async fn get_transactions_by_hashes(
        &self,
        ctx: Context,
        tx_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_transactions_by_scripts(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>>;

    async fn get_block_header(
        &self,
        ctx: Context,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<HeaderView>;

    async fn get_scripts(
        &self,
        ctx: Context,
        script_hashes: Vec<H160>,
        code_hash: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
    ) -> Result<Vec<packed::Script>>;

    async fn get_cells_by_partial_args(
        &self,
        ctx: Context,
        p_lock_args: Option<PartialBytes>,
        p_type_args: Option<PartialBytes>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;

    async fn get_cells_by_partial_data(
        &self,
        ctx: Context,
        p_data: PartialBytes,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>>;
}

pub struct PartialBytes {
    pub content: Vec<u8>,
    pub range: std::ops::Range<usize>,
}
