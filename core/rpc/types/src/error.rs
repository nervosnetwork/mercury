use common::derive_more::Display;

use jsonrpsee_http_server::types::Error;
use serde::{Deserialize, Serialize};

pub trait RpcError {
    fn err_code(&self) -> i32;
    fn message(&self) -> String;
}

#[allow(clippy::from_over_into)]
impl Into<Error> for Box<dyn RpcError> {
    fn into(self) -> Error {
        Error::Custom(format!("Error({}): {:?}", self.err_code(), self.message()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum TypeError {
    #[display(fmt = "Decode json error {}", _0)]
    DecodeJson(String),

    #[display(fmt = "Decode hex string error {}", _0)]
    DecodeHex(String),

    #[display(fmt = "Invalid record id {}", _0)]
    InvalidRecordID(String),
}

impl RpcError for TypeError {
    fn err_code(&self) -> i32 {
        match self {
            TypeError::DecodeJson(_) => -11000,
            TypeError::DecodeHex(_) => -11001,
            TypeError::InvalidRecordID(_) => -11002,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}
