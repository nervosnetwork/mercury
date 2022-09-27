mod fetch;
mod insert;
mod remove;
mod snowflake;

#[cfg(test)]
mod tests;

use crate::relational::{
    fetch::bytes_to_h256, fetch::to_pagination_response, snowflake::Snowflake,
};
use crate::{error::DBError, Storage};
pub use insert::{
    bulk_insert_blocks, bulk_insert_output_cells, bulk_insert_transactions,
    push_values_placeholders, BATCH_SIZE_THRESHOLD, BLAKE_160_HSAH_LEN, IO_TYPE_INPUT,
    IO_TYPE_OUTPUT,
};
use remove::remove_block_table;

use common::{
    async_trait, DetailedCell, Order, PaginationRequest, PaginationResponse, Range, Result,
};
use core_rpc_types::indexer::Transaction;
use db_sqlx::{build_next_cursor, SQLXPool};
use protocol::db::{DBDriver, DBInfo, SimpleBlock, SimpleTransaction, TransactionWrapper};

use ckb_jsonrpc_types::Script;
use ckb_types::core::{BlockNumber, BlockView, HeaderView};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use sqlx::Row;

use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref SNOWFLAKE: Snowflake = Snowflake::default();
}

#[derive(Clone, Debug)]
pub struct RelationalStorage {
    pub sqlx_pool: SQLXPool,
}

#[async_trait]
impl Storage for RelationalStorage {
    async fn append_block(&self, block: BlockView) -> Result<()> {
        let mut tx = self.sqlx_pool.transaction().await?;
        self.insert_block_table(&block, &mut tx).await?;
        self.insert_transaction_table(&block, &mut tx).await?;
        tx.commit().await.map_err(Into::into)
    }

    async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()> {
        let mut tx = self.sqlx_pool.transaction().await?;
        self.remove_tx_and_cell(block_number, block_hash.clone(), &mut tx)
            .await?;
        remove_block_table(block_number, block_hash, &mut tx).await?;
        tx.commit().await.map_err(Into::into)
    }

    async fn get_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        self.query_cells(
            out_point,
            lock_hashes,
            type_hashes,
            block_range,
            false,
            pagination,
        )
        .await
    }

    async fn get_live_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        self.query_live_cells(out_point, lock_hashes, type_hashes, block_range, pagination)
            .await
    }

    async fn get_live_cells_ex(
        &self,
        lock_script: Option<Script>,
        type_script: Option<Script>,
        lock_len_range: Option<Range>,
        type_len_range: Option<Range>,
        block_range: Option<Range>,
        capacity_range: Option<Range>,
        data_len_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        self.query_live_cells_ex(
            lock_script,
            type_script,
            lock_len_range,
            type_len_range,
            block_range,
            capacity_range,
            data_len_range,
            pagination,
        )
        .await
    }

    async fn get_historical_live_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        tip_block_number: BlockNumber,
        out_point: Option<packed::OutPoint>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        if lock_hashes.is_empty() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query historical live cells".to_owned(),
            )
            .into());
        }
        self.query_historical_live_cells(
            lock_hashes,
            type_hashes,
            tip_block_number,
            out_point,
            pagination,
        )
        .await
    }

    async fn get_transactions(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        limit_cellbase: bool,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>> {
        if out_point.is_none()
            && lock_hashes.is_empty()
            && type_hashes.is_empty()
            && block_range.is_none()
        {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }
        let mut set = HashSet::new();
        if !lock_hashes.is_empty() || !type_hashes.is_empty() || out_point.is_some() {
            for cell in self
                .query_cells(
                    out_point,
                    lock_hashes,
                    type_hashes,
                    block_range.clone(),
                    limit_cellbase,
                    Default::default(),
                )
                .await?
                .response
            {
                set.insert(cell.out_point.tx_hash().unpack());
                if let Some(hash) = &cell.consumed_tx_hash {
                    set.insert(hash.to_owned());
                }
            }
            if set.is_empty() {
                return Ok(PaginationResponse {
                    response: vec![],
                    next_cursor: None,
                    count: if pagination.return_count {
                        Some(0)
                    } else {
                        None
                    },
                });
            }
        }

        let tx_hashes = set.into_iter().collect();
        let tx_tables = self
            .query_transactions(tx_hashes, block_range, pagination)
            .await?;
        let txs_wrapper = self
            .get_transactions_with_status(tx_tables.response)
            .await?;
        let next_cursor: Option<u64> = tx_tables.next_cursor.map(Into::into);

        Ok(to_pagination_response(
            txs_wrapper,
            next_cursor,
            tx_tables.count.map(Into::into),
        ))
    }

    async fn get_transactions_by_hashes(
        &self,
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
        let tx_tables = self
            .query_transactions(tx_hashes, block_range, pagination)
            .await?;
        let txs_wrapper = self
            .get_transactions_with_status(tx_tables.response)
            .await?;
        let next_cursor: Option<u64> = tx_tables.next_cursor.map(Into::into);
        Ok(to_pagination_response(
            txs_wrapper,
            next_cursor,
            tx_tables.count.map(Into::into),
        ))
    }

    async fn get_transactions_by_scripts(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        limit_cellbase: bool,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionWrapper>> {
        if block_range.is_none() && lock_hashes.is_empty() && type_hashes.is_empty() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        let tx_hashes = self
            .query_transaction_hashes_by_scripts(
                &lock_hashes,
                &type_hashes,
                &block_range,
                limit_cellbase,
                &pagination,
            )
            .await?;

        let count = if pagination.return_count {
            let count = self
                .query_distinct_tx_hashes_count(
                    &lock_hashes,
                    &type_hashes,
                    &block_range,
                    limit_cellbase,
                )
                .await?;
            Some(count)
        } else {
            None
        };

        if tx_hashes.is_empty() {
            return Ok(PaginationResponse {
                response: vec![],
                next_cursor: None,
                count: count.map(Into::into),
            });
        }

        let next_cursor = if tx_hashes.is_empty() {
            None
        } else {
            build_next_cursor(
                pagination.limit.unwrap_or(u16::MAX),
                tx_hashes.last().unwrap().1,
                tx_hashes.len(),
                count,
            )
        };

        let pag = if pagination.order.is_asc() {
            PaginationRequest::default()
        } else {
            PaginationRequest::default().order(Order::Desc)
        };

        let tx_hashes = tx_hashes.into_iter().map(|(tx_hash, _)| tx_hash).collect();
        let tx_tables = self.query_transactions(tx_hashes, block_range, pag).await?;
        let txs_wrapper = self
            .get_transactions_with_status(tx_tables.response)
            .await?;
        Ok(fetch::to_pagination_response(
            txs_wrapper,
            next_cursor,
            count,
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
    ) -> Result<Vec<packed::Script>> {
        self.query_scripts(script_hashes, code_hashes, args_len, args)
            .await
    }

    async fn get_tip(&self) -> Result<Option<(BlockNumber, H256)>> {
        self.query_tip().await
    }

    async fn get_spent_transaction_hash(
        &self,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>> {
        self.query_spent_tx_hash(out_point).await
    }

    async fn get_canonical_block_hash(&self, block_number: BlockNumber) -> Result<H256> {
        self.query_canonical_block_hash(block_number).await
    }

    async fn get_simple_transaction_by_hash(&self, tx_hash: H256) -> Result<SimpleTransaction> {
        self.query_simple_transaction(tx_hash).await
    }

    async fn get_scripts_by_partial_arg(
        &self,
        code_hash: &H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>> {
        let offset = i32::try_from(offset_location.0 + 1)?;
        let len = i32::try_from(offset_location.1 - offset_location.0)?;
        let query = SQLXPool::new_query(
            r#"
            SELECT script_code_hash, script_args, script_type 
            FROM mercury_script
            WHERE script_code_hash = $1 
            AND substring(script_args, $3, $4) = $2
            "#,
        )
        .bind(code_hash.as_bytes())
        .bind(arg.to_vec())
        .bind(offset)
        .bind(len);
        let res = self.sqlx_pool.fetch(query).await?;
        Ok(res
            .into_iter()
            .map(|row| {
                packed::ScriptBuilder::default()
                    .code_hash(bytes_to_h256(row.get("script_code_hash")).pack())
                    .args(row.get::<Vec<u8>, _>("script_args").pack())
                    .hash_type(packed::Byte::new(row.get::<i16, _>("script_type") as u8))
                    .build()
            })
            .collect())
    }

    async fn get_registered_address(&self, lock_hash: H160) -> Result<Option<String>> {
        self.query_registered_address(lock_hash.as_bytes()).await
    }

    async fn register_addresses(&self, addresses: Vec<(H160, String)>) -> Result<Vec<H160>> {
        self.bulk_insert_registered_address_table(addresses).await
    }

    fn get_db_info(&self) -> Result<DBInfo> {
        let info = SNOWFLAKE.get_info();

        Ok(DBInfo {
            version: clap::crate_version!().to_string(),
            db: DBDriver::PostgreSQL,
            conn_size: self.sqlx_pool.get_max_connections(),
            center_id: info.0,
            machine_id: info.1,
        })
    }

    async fn get_simple_block(
        &self,
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

    async fn get_indexer_transactions(
        &self,
        lock_script: Option<Script>,
        type_script: Option<Script>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<Transaction>> {
        self.query_indexer_transactions(lock_script, type_script, block_range, pagination)
            .await
    }

    async fn indexer_synced_count(&self) -> Result<u64> {
        self.sqlx_pool.fetch_count("mercury_sync_status").await
    }

    async fn block_count(&self) -> Result<u64> {
        self.sqlx_pool.fetch_count("mercury_block").await
    }
}

impl RelationalStorage {
    pub fn new(
        center_id: u16,
        machine_id: u16,
        max_connections: u32,
        min_connections: u32,
        connect_timeout: u64,
        max_lifetime: u64,
        idle_timeout: u64,
    ) -> Self {
        let sqlx_pool = SQLXPool::new(
            center_id,
            machine_id,
            max_connections,
            min_connections,
            connect_timeout,
            max_lifetime,
            idle_timeout,
        );
        RelationalStorage { sqlx_pool }
    }

    pub async fn connect(
        &mut self,
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.sqlx_pool
            .connect(&db_driver, db_name, host, port, user, password)
            .await?;
        Ok(())
    }

    pub fn get_pool(&self) -> SQLXPool {
        self.sqlx_pool.clone()
    }

    pub async fn get_tip_number(&self) -> Result<BlockNumber> {
        let query =
            SQLXPool::new_query("SELECT MAX(block_number) AS tip FROM mercury_canonical_chain");
        let res = self.sqlx_pool.fetch_optional(query).await?;
        res.map(|row| row.get::<i32, _>("tip") as u64)
            .ok_or_else(|| DBError::NotExist("genesis block".to_string()).into())
    }
}

pub fn generate_id(block_number: BlockNumber) -> i64 {
    let number = block_number as i64;
    SNOWFLAKE.generate(number)
}
