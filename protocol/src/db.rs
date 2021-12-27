use common::{DetailedCell, Result};

use ckb_types::core::{BlockNumber, RationalU256, TransactionView};
use ckb_types::{packed, H256};

use ckb_jsonrpc_types::TransactionWithStatus;
use serde::{Deserialize, Serialize};

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

pub struct ConsumeInfo {
    pub output_point: packed::OutPoint,
    pub since: u64,
    pub input_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: H256,
    pub consumed_tx_hash: H256,
    pub consumed_tx_index: u32,
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
