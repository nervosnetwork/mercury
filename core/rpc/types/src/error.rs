use common::derive_more::Display;

use jsonrpsee_core::Error;
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display};

pub trait RpcError: Debug + Display {
    fn err_code(&self) -> i32;
    fn message(&self) -> String;
}

#[derive(Debug, Display)]
pub struct MercuryRpcError(pub Box<dyn RpcError + Send>);

#[allow(clippy::from_over_into)]
impl From<MercuryRpcError> for Error {
    fn from(err: MercuryRpcError) -> Error {
        Error::Custom(format!(
            "Error({}): {:?}",
            err.0.err_code(),
            err.0.message()
        ))
    }
}

impl std::error::Error for MercuryRpcError {}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum TypeError {
    #[display(fmt = "Decode json error {}", _0)]
    DecodeJson(String),

    #[display(fmt = "Decode hex string error {}", _0)]
    DecodeHex(String),

    #[display(fmt = "Invalid record id {}", _0)]
    InvalidRecordID(String),

    #[display(fmt = "Unsupport identity flag {}", _0)]
    UnsupportIdentityFlag(u8),
}

impl RpcError for TypeError {
    fn err_code(&self) -> i32 {
        match self {
            TypeError::DecodeJson(_) => -12000,
            TypeError::DecodeHex(_) => -12001,
            TypeError::InvalidRecordID(_) => -12002,
            TypeError::UnsupportIdentityFlag(_) => -12003,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<TypeError> for MercuryRpcError {
    fn from(err: TypeError) -> Self {
        MercuryRpcError(Box::new(err))
    }
}
