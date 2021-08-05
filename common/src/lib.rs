pub mod address;
pub mod hash;
pub mod utils;

pub use address::{Address, AddressPayload, AddressType, CodeHashIndex};
pub use {anyhow, async_trait::async_trait, derive_more};

use ckb_types::{bytes::Bytes, h256, H256};
use derive_more::Display;
use serde_derive::{Deserialize, Serialize};

use std::fmt::{self, Debug, Display};

pub const SIGHASH_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
pub const MULTISIG_TYPE_HASH: H256 =
    h256!("0x5c5069eb0857efc65e1bca0c07df34c31663b3622fd3876c876320fc9634e2a8");
pub const DAO_TYPE_HASH: H256 =
    h256!("0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e");
pub const ACP_MAINNET_TYPE_HASH: H256 =
    h256!("0xd369597ff47f29fbc0d47d2e3775370d1250b85140c670e4718af712983a2354");
pub const ACP_TESTNET_TYPE_HASH: H256 =
    h256!("0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356");
pub const PREFIX_MAINNET: &str = "ckb";
pub const PREFIX_TESTNET: &str = "ckt";
pub const NETWORK_MAINNET: &str = "ckb";
pub const NETWORK_TESTNET: &str = "ckb_testnet";
pub const NETWORK_STAGING: &str = "ckb_staging";
pub const NETWORK_DEV: &str = "ckb_dev";

#[derive(Clone, Debug, PartialEq, Eq)]
enum ErrorKind {
    Cli,
    DB,
    Extension,
    Rpc,
    Service,
    Storage,
    Utils,
}

#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(fmt = "Mercury {:?} Error {:?}", kind, error)]
pub struct MercuryError<T> {
    kind: ErrorKind,
    error: T,
}

impl<T: Debug + Display> std::error::Error for MercuryError<T> {}

impl<T: Debug + Display> MercuryError<T> {
    pub fn cli(error: T) -> Self {
        Self::new(ErrorKind::Cli, error)
    }

    pub fn db(error: T) -> Self {
        Self::new(ErrorKind::DB, error)
    }

    pub fn extension(error: T) -> Self {
        Self::new(ErrorKind::Extension, error)
    }

    pub fn rpc(error: T) -> Self {
        Self::new(ErrorKind::Rpc, error)
    }

    pub fn service(error: T) -> Self {
        Self::new(ErrorKind::Service, error)
    }

    pub fn storage(error: T) -> Self {
        Self::new(ErrorKind::Storage, error)
    }

    pub fn utils(error: T) -> Self {
        Self::new(ErrorKind::Utils, error)
    }

    fn new(kind: ErrorKind, error: T) -> Self {
        MercuryError { kind, error }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Staging,
    Dev,
}

impl NetworkType {
    pub fn from_prefix(value: &str) -> Option<NetworkType> {
        match value {
            PREFIX_MAINNET => Some(NetworkType::Mainnet),
            PREFIX_TESTNET => Some(NetworkType::Testnet),
            _ => None,
        }
    }

    pub fn to_prefix(self) -> &'static str {
        match self {
            NetworkType::Mainnet => PREFIX_MAINNET,
            NetworkType::Testnet => PREFIX_TESTNET,
            NetworkType::Staging => PREFIX_TESTNET,
            NetworkType::Dev => PREFIX_TESTNET,
        }
    }

    pub fn from_raw_str(value: &str) -> Option<NetworkType> {
        match value {
            NETWORK_MAINNET => Some(NetworkType::Mainnet),
            NETWORK_TESTNET => Some(NetworkType::Testnet),
            NETWORK_STAGING => Some(NetworkType::Staging),
            NETWORK_DEV => Some(NetworkType::Dev),
            _ => None,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            NetworkType::Mainnet => NETWORK_MAINNET,
            NetworkType::Testnet => NETWORK_TESTNET,
            NetworkType::Staging => NETWORK_STAGING,
            NetworkType::Dev => NETWORK_DEV,
        }
    }
}

impl fmt::Display for NetworkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Range {
    pub from: u64,
    pub to: u64,
}

impl Range {
    pub fn new(from: u64, to: u64) -> Self {
        Range { from, to }
    }

    pub fn to_usize(&self) -> (usize, usize) {
        (self.from as usize, self.to as usize)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Pagination {
    pub cursor: Bytes,
    pub order: Order,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationResponse<T> {
    response: Vec<T>,
    next_cursot: Option<Bytes>,
    count: Option<u64>,
}
