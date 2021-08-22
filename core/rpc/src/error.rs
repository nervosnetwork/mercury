use common::derive_more::Display;

use jsonrpsee_http_server::types::{CallError, Error};
use serde::{Deserialize, Serialize};

pub type RpcResult<T> = Result<T, Error>;

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "Mercury Rpc Error code {}, error {}", err_code, message)]
pub struct RpcError {
    err_code: i32,
    message: String,
}

impl Into<Error> for RpcError {
    fn into(self) -> Error {
        Error::Call(CallError::Failed(self.into()))
    }
}

impl RpcError {
    pub fn new(err_code: i32, message: String) -> Self {
        RpcError { err_code, message }
    }
}

impl std::error::Error for RpcError {}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum RpcErrorMessage {}
