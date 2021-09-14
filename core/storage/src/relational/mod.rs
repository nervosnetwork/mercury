mod fetch;
mod insert;
mod remove;
mod snowflake;
mod sql;
pub mod table;

#[cfg(test)]
mod tests;

use snowflake::Snowflake;
use table::BsonBytes;

use crate::{error::DBError, Storage};

use common::{
    async_trait, utils::to_fixed_array, DetailedCell, PaginationRequest, PaginationResponse, Range,
    Result,
};
use db_protocol::{DBDriver, DBInfo, SimpleBlock, SimpleTransaction};
use db_xsql::XSQLPool;

use bson::spec::BinarySubtype;
use ckb_types::core::{BlockNumber, BlockView, HeaderView, TransactionView};
use ckb_types::{bytes::Bytes, packed, H160, H256};
use log::LevelFilter;
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
    async fn append_block(&self, block: BlockView) -> Result<()> {
        let mut tx = self.pool.transaction().await?;

        self.insert_block_table(&block, &mut tx).await?;
        self.insert_transaction_table(&block, &mut tx).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn rollback_block(&self, block_number: BlockNumber, block_hash: H256) -> Result<()> {
        let mut tx = self.pool.transaction().await?;
        let block_hash = to_bson_bytes(&block_hash.0);

        self.remove_tx_and_cell(block_number, block_hash.clone(), &mut tx)
            .await?;
        self.remove_consume_info(block_number, block_hash.clone(), &mut tx)
            .await?;
        self.remove_canonical_chain(block_number, block_hash, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    async fn get_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();

        self.query_cells(
            out_point,
            lock_hashes,
            type_hashes,
            block_number,
            block_range,
            pagination,
        )
        .await
    }

    async fn get_live_cells(
        &self,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_number: Option<BlockNumber>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(&hash.0))
            .collect::<Vec<_>>();

        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(&hash.0))
            .collect::<Vec<_>>();

        self.query_live_cells(
            out_point,
            lock_hashes,
            type_hashes,
            block_number,
            block_range,
            pagination,
        )
        .await
    }

    async fn get_transactions(
        &self,
        tx_hashes: Vec<H256>,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<TransactionView>> {
        let tx_hashes = tx_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let lock_hashes = lock_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let type_hashes = type_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let tx_tables = self
            .query_transactions(tx_hashes, lock_hashes, type_hashes, block_range, pagination)
            .await?;
        let tx_views = self.get_transaction_views(tx_tables.response).await?;
        let next_cursor = tx_tables.next_cursor.map(|bytes| {
            i64::from_be_bytes(
                bytes
                    .to_vec()
                    .try_into()
                    .expect("slice with incorrect length"),
            )
        });
        Ok(fetch::to_pagination_response(
            tx_views,
            next_cursor,
            tx_tables.count.unwrap_or(0),
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
        let script_hashes = script_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let code_hashes = code_hashes
            .into_iter()
            .map(|hash| to_bson_bytes(hash.as_bytes()))
            .collect::<Vec<_>>();
        let args = args
            .into_iter()
            .map(|arg| to_bson_bytes(&arg))
            .collect::<Vec<_>>();

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
        code_hash: H256,
        arg: Bytes,
        offset_location: (u32, u32),
    ) -> Result<Vec<packed::Script>> {
        let mut conn = self.pool.acquire().await?;
        let offset = offset_location.0 + 1;
        let len = offset_location.1 - offset_location.0;

        let ret = sql::query_scripts_by_partial_arg(
            &mut conn,
            to_bson_bytes(&code_hash.0),
            to_bson_bytes(&arg),
            offset,
            len,
        )
        .await?;
        Ok(ret.into_iter().map(Into::into).collect())
    }

    async fn get_registered_address(&self, lock_hash: H160) -> Result<Option<String>> {
        let lock_hash = to_bson_bytes(lock_hash.as_bytes());
        let res = self.query_registered_address(lock_hash).await?;
        Ok(res.map(|t| t.address))
    }

    async fn register_addresses(&self, addresses: Vec<(H160, String)>) -> Result<Vec<H160>> {
        let mut tx = self.pool.transaction().await?;
        let addresses = addresses
            .into_iter()
            .map(|(lock_hash, address)| (to_bson_bytes(lock_hash.as_bytes()), address))
            .collect::<Vec<_>>();
        let res = self
            .insert_registered_address_table(addresses, &mut tx)
            .await?;
        tx.commit().await?;

        Ok(res
            .iter()
            .map(|hash| H160(to_fixed_array::<HASH160_LEN>(&hash.bytes)))
            .collect())
    }

    fn get_db_info(&self) -> Result<DBInfo> {
        let info = SNOWFLAKE.get_info();

        Ok(DBInfo {
            version: clap::crate_version!().to_string(),
            db: DBDriver::PostgreSQL,
            conn_size: self.pool.get_config().max_connections,
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
            .fetch_count_by_wrapper::<table::BlockTable>(&w)
            .await?;
        Ok(ret)
    }
}

pub fn generate_id(block_number: BlockNumber) -> i64 {
    let number = block_number as i64;
    SNOWFLAKE.generate(number)
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
