pub mod address;
pub mod hash;
pub mod lazy;
pub mod utils;

pub use address::{Address, AddressPayload, AddressType, CodeHashIndex};
pub use {anyhow, anyhow::Result, async_trait::async_trait, derive_more, minstant};

use ckb_types::{h256, H256};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use std::fmt::{self, Debug, Display};

pub const MULTISIG_TYPE_HASH: H256 =
    h256!("0x5c5069eb0857efc65e1bca0c07df34c31663b3622fd3876c876320fc9634e2a8");

pub const PREFIX_MAINNET: &str = "ckb";
pub const PREFIX_TESTNET: &str = "ckt";
pub const NETWORK_MAINNET: &str = "ckb";
pub const NETWORK_TESTNET: &str = "ckb_testnet";
pub const NETWORK_STAGING: &str = "ckb_staging";
pub const NETWORK_DEV: &str = "ckb_dev";

pub const SECP256K1: &str = "secp256k1_blake160";
pub const SUDT: &str = "sudt";
pub const ACP: &str = "anyone_can_pay";
pub const CHEQUE: &str = "cheque";
pub const DAO: &str = "dao";
pub const PW_LOCK: &str = "pw_lock";

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
pub enum Order {
    #[serde(alias = "asc")]
    Asc,
    #[serde(alias = "desc")]
    Desc,
}

impl Default for Order {
    fn default() -> Self {
        Order::Asc
    }
}

impl Order {
    pub fn is_asc(&self) -> bool {
        *self == Order::Asc
    }
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

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "range from {} to {}", from, to)]
pub struct Range {
    pub from: u64,
    pub to: u64,
}

impl Range {
    pub fn new(from: u64, to: u64) -> Self {
        Range { from, to }
    }

    pub fn is_in(&self, num: u64) -> bool {
        num >= self.from && num <= self.to
    }

    pub fn min(&self) -> u64 {
        self.from
    }

    pub fn max(&self) -> u64 {
        self.to
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationRequest {
    pub cursor: Option<u64>,
    pub order: Order,
    pub limit: Option<u16>,
    pub skip: Option<u64>,
    pub return_count: bool,
}

impl PaginationRequest {
    pub fn new(
        cursor: Option<u64>,
        order: Order,
        limit: Option<u16>,
        skip: Option<u64>,
        return_count: bool,
    ) -> PaginationRequest {
        PaginationRequest {
            cursor,
            order,
            limit,
            skip,
            return_count,
        }
    }

    pub fn order(mut self, order: Order) -> Self {
        self.set_order(order);
        self
    }

    pub fn set_order(&mut self, order: Order) {
        self.order = order;
    }

    pub fn limit(mut self, limit: Option<u16>) -> Self {
        self.set_limit(limit);
        self
    }

    pub fn set_limit(&mut self, limit: Option<u16>) {
        self.limit = limit;
    }

    pub fn update_by_response(&mut self, next_cursor: Option<u64>) {
        self.cursor = next_cursor;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationResponse<T> {
    pub response: Vec<T>,
    pub next_cursor: Option<u64>,
    pub count: Option<u64>,
}

impl<T> PaginationResponse<T> {
    pub fn new(response: Vec<T>) -> Self {
        Self {
            response,
            next_cursor: None,
            count: None,
        }
    }
}

impl<T> Default for PaginationResponse<T> {
    fn default() -> Self {
        Self::new(vec![])
    }
}

pub fn display_list_as_hex<T: AsRef<[u8]>>(list: &[T]) {
    list.iter()
        .for_each(|i| println!("{:?}", hex::encode(i.as_ref())))
}
