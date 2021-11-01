mod fetch;
mod insert;
mod remove;
mod snowflake;
mod sql;
pub mod table;

#[cfg(test)]
mod tests;

pub use insert::BATCH_SIZE_THRESHOLD;

use crate::relational::{
    fetch::to_pagination_response, snowflake::Snowflake, table::IndexerCellTable,
};
use crate::{error::DBError, Storage};

use common::{
    async_trait, utils::to_fixed_array, Context, DetailedCell, Order, PaginationRequest,
    PaginationResponse, Range, Result,
};
use common_logger::{tracing, tracing_async};
use db_protocol::{DBDriver, DBInfo, SimpleBlock, SimpleTransaction, TransactionWrapper};
use db_xsql::{rbatis::core::types::byte::RbBytes, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView, HeaderView};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use log::LevelFilter;

use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::HashSet;
use std::convert::TryInto;

const HASH160_LEN: usize = 20;

lazy_static::lazy_static! {
    pub static ref SNOWFLAKE: Snowflake = Snowflake::default();
}

#[derive(Clone, Debug)]
pub struct RelationalStorage {
    pub pool: XSQLPool,
}

#[async_trait]
impl Storage for RelationalStorage {
    #[tracing_async]
    async fn append_block(&self, ctx: Context, block: BlockView) -> Result<()> {
        let mut tx = self.pool.transaction().await?;
        self.insert_block_table(ctx.clone(), &block, &mut tx)
            .await?;
        self.insert_transaction_table(ctx.clone(), &block, &mut tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    #[tracing_async]
    async fn rollback_block(
        &self,
        ctx: Context,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<()> {
        let mut tx = self.pool.transaction().await?;
        let block_hash = to_rb_bytes(&block_hash.0);

        self.remove_tx_and_cell(ctx.clone(), block_number, block_hash.clone(), &mut tx)
            .await?;
        self.remove_block_table(ctx.clone(), block_number, block_hash, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    #[tracing_async]
    async fn get_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        self.query_cells(
            ctx,
            out_point,
            lock_hashes,
            type_hashes,
            block_range,
            pagination,
        )
        .await
    }

    #[tracing_async]
    async fn get_live_cells(
        &self,
        ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(&hash.0))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(&hash.0))
            .collect::<Vec<_>>();

        self.query_live_cells(
            ctx,
            out_point,
            lock_hashes,
            type_hashes,
            block_range,
            pagination,
        )
        .await
    }

    #[tracing_async]
    async fn get_historical_live_cells(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: BlockNumber,
    ) -> Result<Vec<DetailedCell>> {
        if lock_hashes.is_empty() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(&hash.0))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(&hash.0))
            .collect::<Vec<_>>();

        self.query_historical_live_cells(ctx, lock_hashes, type_hashes, tip_block_number)
            .await
    }

    #[tracing_async]
    async fn get_transactions(
        &self,
        ctx: Context,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>> {
        if tx_hashes.is_empty()
            && block_range.is_none()
            && lock_hashes.is_empty()
            && type_hashes.is_empty()
        {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        let mut tx_hashes = tx_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        if !lock_hashes.is_empty() || !type_hashes.is_empty() {
            let mut set = HashSet::new();
            for cell in self
                .query_cells(
                    ctx.clone(),
                    None,
                    lock_hashes,
                    type_hashes,
                    block_range.clone(),
                    Default::default(),
                )
                .await?
                .response
                .iter()
            {
                set.insert(cell.out_point.tx_hash().raw_data().to_vec());
                if let Some(hash) = &cell.consumed_tx_hash {
                    set.insert(hash.0.to_vec());
                }
            }

            tx_hashes.extend(set.iter().map(|bytes| to_rb_bytes(bytes)));
        }

        let tx_tables = self
            .query_transactions(ctx.clone(), tx_hashes, block_range, pagination)
            .await?;
        let txs_wrapper = self
            .get_transactions_with_status(ctx, tx_tables.response)
            .await?;
        let next_cursor = tx_tables.next_cursor.map(|bytes| {
            i64::from_be_bytes(
                bytes
                    .to_vec()
                    .try_into()
                    .expect("slice with incorrect length"),
            )
        });

        Ok(to_pagination_response(
            txs_wrapper,
            next_cursor,
            tx_tables.count.unwrap_or(0),
        ))
    }

    #[tracing_async]
    async fn get_transactions_by_hashes(
        &self,
        ctx: Context,
        tx_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>> {
        if tx_hashes.is_empty() && block_range.is_none() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        let tx_hashes = tx_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(&hash.0))
            .collect::<Vec<_>>();
        let tx_tables = self
            .query_transactions(ctx.clone(), tx_hashes, block_range, pagination)
            .await?;
        let txs_wrapper = self
            .get_transactions_with_status(ctx.clone(), tx_tables.response)
            .await?;
        let next_cursor = tx_tables.next_cursor.map(|bytes| {
            i64::from_be_bytes(
                bytes
                    .to_vec()
                    .try_into()
                    .expect("slice with incorrect length"),
            )
        });

        Ok(to_pagination_response(
            txs_wrapper,
            next_cursor,
            tx_tables.count.unwrap_or(0),
        ))
    }

    #[tracing_async]
    async fn get_transactions_by_scripts(
        &self,
        ctx: Context,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>> {
        if block_range.is_none() && lock_hashes.is_empty() && type_hashes.is_empty() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        let mut set = HashSet::new();
        for cell in self
            .query_cells(
                ctx.clone(),
                None,
                lock_hashes,
                type_hashes,
                block_range.clone(),
                Default::default(),
            )
            .await?
            .response
            .iter()
        {
            set.insert(TxHashInfo::new(
                cell.out_point.tx_hash().unpack(),
                cell.block_number,
                cell.tx_index,
            ));
            if let Some(hash) = &cell.consumed_tx_hash {
                set.insert(TxHashInfo::new(
                    hash.clone(),
                    cell.consumed_block_number.unwrap(),
                    cell.consumed_tx_index.unwrap(),
                ));
            }
        }

        let mut cells = set.into_iter().collect::<Vec<_>>();
        cells.sort();
        if pagination.order == Order::Desc {
            cells.reverse();
        }

        let limit = if let Some(pgn_limit) = pagination.limit {
            if pgn_limit > 60000 {
                60000
            } else {
                pgn_limit as usize
            }
        } else {
            60000
        };

        let from = if let Some(cursor) = pagination.cursor.clone() {
            let id = i64::from_be_bytes(to_fixed_array(&cursor));
            let tx = self
                .query_transaction_by_id(id, pagination.order.is_asc())
                .await?;
            (tx.block_number, tx.tx_index)
        } else {
            (0, 0)
        };

        let tx_hashes = if pagination.order == Order::Asc {
            cells
                .into_iter()
                .filter_map(|cell| {
                    if cell.block_number > from.0
                        || (cell.block_number == from.0 && cell.tx_index >= from.1)
                    {
                        Some(to_rb_bytes(&cell.hash.0))
                    } else {
                        None
                    }
                })
                .take(limit)
                .collect::<Vec<_>>()
        } else {
            cells
                .into_iter()
                .filter_map(|cell| {
                    if cell.block_number < from.0
                        || (cell.block_number == from.0 && cell.tx_index <= from.1)
                    {
                        Some(to_rb_bytes(&cell.hash.0))
                    } else {
                        None
                    }
                })
                .take(limit)
                .collect::<Vec<_>>()
        };

        let tx_tables = self
            .query_transactions(ctx.clone(), tx_hashes, block_range, pagination)
            .await?;
        let txs_wrapper = self
            .get_transactions_with_status(ctx, tx_tables.response)
            .await?;
        let next_cursor = tx_tables.next_cursor.map(|bytes| {
            i64::from_be_bytes(
                bytes
                    .to_vec()
                    .try_into()
                    .expect("slice with incorrect length"),
            )
        });

        Ok(fetch::to_pagination_response(
            txs_wrapper,
            next_cursor,
            tx_tables.count.unwrap_or(0),
        ))
    }

    #[tracing_async]
    async fn get_block(
        &self,
        ctx: Context,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<BlockView> {
        match (block_hash, block_number) {
            (None, None) => self.get_tip_block(ctx).await,
            (None, Some(block_number)) => self.get_block_by_number(ctx, block_number).await,
            (Some(block_hash), None) => self.get_block_by_hash(ctx, block_hash).await,
            (Some(block_hash), Some(block_number)) => {
                let result = self.get_block_by_hash(ctx, block_hash).await;
                if let Ok(ref block_view) = result {
                    if block_view.number() != block_number {
                        return Err(DBError::MismatchBlockHash.into());
                    }
                }
                result
            }
        }
    }

    #[tracing_async]
    async fn get_block_header(
        &self,
        _ctx: Context,
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

    #[tracing_async]
    async fn get_scripts(
        &self,
        _ctx: Context,
        script_hashes: Vec<H160>,
        code_hashes: Vec<H256>,
        args_len: Option<usize>,
        args: Vec<Bytes>,
    ) -> Result<Vec<packed::Script>> {
        let script_hashes = script_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let code_hashes = code_hashes
            .into_iter()
            .map(|hash| to_rb_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let args = args
            .into_iter()
            .map(|arg| to_rb_bytes(&arg))
            .collect::<Vec<_>>();

        self.query_scripts(script_hashes, code_hashes, args_len, args)
            .await
    }

    #[tracing_async]
    async fn get_tip(&self, _ctx: Context) -> Result<Option<(BlockNumber, H256)>> {
        self.query_tip().await
    }

    #[tracing_async]
    async fn get_spent_transaction_hash(
        &self,
        _ctx: Context,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>> {
        self.query_spent_tx_hash(out_point).await
    }

    #[tracing_async]
    async fn get_canonical_block_hash(
        &self,
        _ctx: Context,
        block_number: BlockNumber,
    ) -> Result<H256> {
        self.query_canonical_block_hash(block_number).await
    }

    #[tracing_async]
    async fn get_simple_transaction_by_hash(
        &self,
        _ctx: Context,
        tx_hash: H256,
    ) -> Result<SimpleTransaction> {
        self.query_simple_transaction(tx_hash).await
    }

    #[tracing_async]
    async fn get_scripts_by_partial_arg(
        &self,
        _ctx: Context,
        code_hash: H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>> {
        let mut conn = self.pool.acquire().await?;
        let offset = offset_location.0 + 1;
        let len = offset_location.1 - offset_location.0;

        let ret = sql::query_scripts_by_partial_arg(
            &mut conn,
            to_rb_bytes(&code_hash.0),
            to_rb_bytes(&arg),
            offset,
            len,
        )
        .await?;

        Ok(ret.into_iter().map(Into::into).collect())
    }

    #[tracing_async]
    async fn get_registered_address(
        &self,
        _ctx: Context,
        lock_hash: H160,
    ) -> Result<Option<String>> {
        let lock_hash = to_rb_bytes(lock_hash.as_bytes());
        let res = self.query_registered_address(lock_hash).await?;
        Ok(res.map(|t| t.address))
    }

    #[tracing_async]
    async fn register_addresses(
        &self,
        _ctx: Context,
        addresses: Vec<(H160, String)>,
    ) -> Result<Vec<H160>> {
        let mut tx = self.pool.transaction().await?;
        let addresses = addresses
            .into_iter()
            .map(|(lock_hash, address)| (to_rb_bytes(lock_hash.as_bytes()), address))
            .collect::<Vec<_>>();
        let res = self
            .insert_registered_address_table(addresses, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(res
            .iter()
            .map(|hash| H160(to_fixed_array::<HASH160_LEN>(&hash.rb_bytes)))
            .collect())
    }

    #[tracing]
    fn get_db_info(&self, _ctx: Context) -> Result<DBInfo> {
        let info = SNOWFLAKE.get_info();

        Ok(DBInfo {
            version: clap::crate_version!().to_string(),
            db: DBDriver::PostgreSQL,
            conn_size: self.pool.get_config().max_connections,
            center_id: info.0,
            machine_id: info.1,
        })
    }

    #[tracing_async]
    async fn get_simple_block(
        &self,
        _ctx: Context,
        block_hash: Option<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<SimpleBlock> {
        match (block_hash, block_number) {
            (None, None) => self.get_tip_simple_block().await,
            (None, Some(block_number)) => self.get_simple_block_by_block_number(block_number).await,
            (Some(block_hash), None) => self.get_simple_block_by_block_hash(block_hash).await,
            (Some(block_hash), Some(block_number)) => {
                let result = self.get_simple_block_by_block_hash(block_hash).await;
                if let Ok(ref block_info) = result {
                    if block_info.block_number != block_number {
                        return Err(DBError::MismatchBlockHash.into());
                    }
                }
                result
            }
        }
    }

    #[tracing_async]
    async fn get_indexer_transactions(
        &self,
        _ctx: Context,
        lock_script: Option<packed::Script>,
        type_script: Option<packed::Script>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<IndexerCellTable>> {
        if lock_script.is_none() && type_script.is_none() && block_range.is_none() {
            return Err(DBError::InvalidParameter(
                "No valid parameter to query indexer cell".to_string(),
            )
            .into());
        }

        self.query_indexer_cells(lock_script, type_script, block_range, pagination)
            .await
    }
}

impl RelationalStorage {
    pub fn new(
        max_connections: u32,
        center_id: u16,
        machine_id: u16,
        log_level: LevelFilter,
    ) -> Self {
        let pool = XSQLPool::new(max_connections, center_id, machine_id, log_level);
        RelationalStorage { pool }
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
        self.pool
            .connect(db_driver, db_name, host, port, user, password)
            .await?;
        Ok(())
    }

    /// This function is provided for test.
    pub fn inner(&self) -> XSQLPool {
        self.pool.clone()
    }

    pub async fn block_count(&self) -> Result<u64> {
        let w = self.pool.wrapper();
        let ret = self
            .pool
            .fetch_count_by_wrapper::<table::BlockTable>(w)
            .await?;
        Ok(ret)
    }
}

pub fn generate_id(block_number: BlockNumber) -> i64 {
    let number = block_number as i64;
    SNOWFLAKE.generate(number)
}

pub fn to_rb_bytes(input: &[u8]) -> RbBytes {
    RbBytes {
        rb_bytes: input.to_vec(),
    }
}

pub fn empty_rb_bytes() -> RbBytes {
    RbBytes::new()
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TxHashInfo {
    hash: H256,
    block_number: u64,
    tx_index: u32,
}

impl TxHashInfo {
    fn new(hash: H256, block_number: u64, tx_index: u32) -> TxHashInfo {
        TxHashInfo {
            hash,
            block_number,
            tx_index,
        }
    }
}

impl Ord for TxHashInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.block_number != other.block_number {
            self.block_number.cmp(&other.block_number)
        } else {
            self.tx_index.cmp(&other.tx_index)
        }
    }
}

impl PartialOrd for TxHashInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
