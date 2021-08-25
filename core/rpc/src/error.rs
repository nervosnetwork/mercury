use common::derive_more::Display;

use jsonrpsee_http_server::types::{CallError, Error};
use serde::{Deserialize, Serialize};

pub type RpcResult<T> = Result<T, Error>;
pub type InnerResult<T> = Result<T, RpcErrorMessage>;

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
#[display(fmt = "Mercury Rpc Error code {}, error {}", err_code, message)]
pub struct RpcError {
    err_code: i32,
    message: String,
}

impl From<RpcErrorMessage> for RpcError {
    fn from(msg: RpcErrorMessage) -> Self {
        RpcError::new(msg.code(), msg.to_string())
    }
}

impl From<RpcError> for Error {
    fn from(error: RpcError) -> Error {
        Error::Call(CallError::Failed(error.into()))
    }
}

impl RpcError {
    pub fn new(err_code: i32, message: String) -> Self {
        RpcError { err_code, message }
    }
}

impl std::error::Error for RpcError {}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum RpcErrorMessage {
    #[display(fmt = "Decode json error {}", _0)]
    DecodeJson(String),

    #[display(fmt = "Ckb client error {}", _0)]
    CkbClientError(String),

    #[display(fmt = "Invalid rpc params {}", _0)]
    InvalidRpcParams(String),

    #[display(fmt = "Get none block from code")]
    GetNoneBlockFromNode,

    #[display(fmt = "Cannot get script by script hash")]
    CannotGetScriptByHash,

    #[display(fmt = "DB error {}", _0)]
    DBError(String),

    #[display(fmt = "Common error {}", _0)]
    CommonError(String),

    #[display(fmt = "Unsupport UDT lock script type")]
    UnsupportUDTLockScript,
}

impl std::error::Error for RpcErrorMessage {}

impl RpcErrorMessage {
    fn code(&self) -> i32 {
        match self {
            RpcErrorMessage::DecodeJson(_) => -11000,
            RpcErrorMessage::CkbClientError(_) => -11001,
            RpcErrorMessage::InvalidRpcParams(_) => -11002,
            RpcErrorMessage::GetNoneBlockFromNode => -11003,
            RpcErrorMessage::CannotGetScriptByHash => -11004,
            RpcErrorMessage::DBError(_) => -11005,
            RpcErrorMessage::CommonError(_) => -11006,
            RpcErrorMessage::UnsupportUDTLockScript => -11007,
        }
    }
}
