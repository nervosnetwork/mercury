use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::H256;
use serde::{Deserialize, Serialize};

pub type IteratorItem = (Box<[u8]>, Box<[u8]>);

pub const MYSQL: &str = "mysql://";
pub const PGSQL: &str = "postgres://";
pub const SQLITE: &str = "sqlite://";

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

#[allow(clippy::from_over_into)]
impl Into<&str> for &DBDriver {
    fn into(self) -> &'static str {
        match self {
            DBDriver::PostgreSQL => PGSQL,
            DBDriver::MySQL => MYSQL,
            DBDriver::SQLite => SQLITE,
        }
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
