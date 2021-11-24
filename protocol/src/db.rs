use common::{DetailedCell, Result};

use ckb_types::core::{BlockNumber, RationalU256, TransactionView};
use ckb_types::{bytes::Bytes, packed, H256};

use ckb_jsonrpc_types::TransactionWithStatus;
use serde::{Deserialize, Serialize};

use std::cmp::Ordering;

pub type IteratorItem = (Box<[u8]>, Box<[u8]>);

pub const MYSQL: &str = "mysql://";
pub const PGSQL: &str = "postgres://";
pub const SQLITE: &str = "sqlite://";

pub enum IteratorDirection {
    Forward,
    Reverse,
}

pub trait KVStore {
    type Batch: KVStoreBatch;

    fn new(path: &str) -> Self;

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>>;

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool>;

    fn iter<K: AsRef<[u8]>>(
        &self,
        from_key: K,
        direction: IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>>;

    fn batch(&self) -> Result<Self::Batch>;
}

pub trait KVStoreBatch {
    fn put_kv<K: Into<Vec<u8>>, V: Into<Vec<u8>>>(&mut self, key: K, value: V) -> Result<()> {
        self.put(&Into::<Vec<u8>>::into(key), &Into::<Vec<u8>>::into(value))
    }

    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<()>;

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<()>;

    fn commit(self) -> Result<()>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SimpleTransaction {
    pub epoch_number: RationalU256,
    pub block_number: BlockNumber,
    pub block_hash: H256,
    pub tx_index: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SimpleBlock {
    pub block_number: BlockNumber,
    pub block_hash: H256,
    pub parent_hash: H256,
    pub timestamp: u64,
    pub transactions: Vec<H256>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ConsumeInfo {
    pub output_point: packed::OutPoint,
    pub since: u64,
    pub input_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: H256,
    pub consumed_tx_hash: H256,
    pub consumed_tx_index: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexerCell {
    pub id: i64,
    pub block_number: u64,
    pub io_type: u8,
    pub io_index: u32,
    pub tx_hash: Bytes,
    pub tx_index: u32,
    pub lock_hash: Bytes,
    pub lock_code_hash: Bytes,
    pub lock_args: Bytes,
    pub lock_script_type: u8,
    pub type_hash: Bytes,
    pub type_code_hash: Bytes,
    pub type_args: Bytes,
    pub type_script_type: u8,
}

impl Ord for IndexerCell {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.block_number != other.block_number {
            self.block_number.cmp(&other.block_number)
        } else if self.tx_index != other.tx_index {
            self.tx_index.cmp(&other.tx_index)
        } else if self.io_type != other.io_type {
            self.io_type.cmp(&other.io_type)
        } else {
            self.io_index.cmp(&other.io_index)
        }
    }
}

impl PartialOrd for IndexerCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct TransactionWrapper {
    pub transaction_with_status: TransactionWithStatus,
    pub transaction_view: TransactionView,
    pub input_cells: Vec<DetailedCell>,
    pub output_cells: Vec<DetailedCell>,
    pub is_cellbase: bool,
    pub timestamp: u64,
}
