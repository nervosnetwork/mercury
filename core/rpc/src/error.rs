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

    #[display(fmt = "Unsupport UDT lock script, code hash {}", _0)]
    UnsupportUDTLockScript(String),

    #[display(fmt = "Decode hex string error {}", _0)]
    DecodeHexError(String),

    #[display(fmt = "{} token is not enough", _0)]
    TokenIsNotEnough(String),

    #[display(fmt = "Cannot find spent transaction")]
    CannotFindSpentTransaction,

    #[display(fmt = "Calcute occupied capacity error {}", _0)]
    OccupiedCapacityError(String),

    #[display(fmt = "Get epoch error of block number {}", _0)]
    GetEpochFromNumberError(u64),

    #[display(fmt = "Adjust account on ckb")]
    AdjustAccountOnCkb,

    #[display(fmt = "Lock hash {} is not registered", _0)]
    LockHashIsNotRegistered(String),

    #[display(fmt = "Need at least one item in from")]
    NeedAtLeastOneFrom,

    #[display(fmt = "Can not find change cell")]
    CannotFindChangeCell,

    #[display(fmt = "Cannot find transaction by hash")]
    CannotFindTransactionByHash,

    #[display(fmt = "Cannot find detailed cell by out point")]
    CannotFindDetailedCellByOutPoint,

    #[display(fmt = "Cannot reference a header less than 4 epochs")]
    CannotReferenceHeader,

    #[display(fmt = "Need at least one item in from and in to")]
    NeedAtLeastOneFromAndOneTo,

    #[display(fmt = "Exceed the maximum item number")]
    ExceedMaxItemNum,

    #[display(fmt = "Required CKB is less than mininum")]
    RequiredCKBLessThanMin,

    #[display(fmt = "Cannot find address by H160")]
    CannotFindAddressByH160,
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
            RpcErrorMessage::UnsupportUDTLockScript(_) => -11007,
            RpcErrorMessage::DecodeHexError(_) => -11008,
            RpcErrorMessage::TokenIsNotEnough(_) => -11009,
            RpcErrorMessage::OccupiedCapacityError(_) => -11010,
            RpcErrorMessage::GetEpochFromNumberError(_) => -11011,
            RpcErrorMessage::LockHashIsNotRegistered(_) => -11012,
            RpcErrorMessage::CannotFindChangeCell => -11013,
            RpcErrorMessage::CannotFindTransactionByHash => -11014,
            RpcErrorMessage::CannotFindDetailedCellByOutPoint => -11015,
            RpcErrorMessage::CannotReferenceHeader => -11016,
            RpcErrorMessage::ExceedMaxItemNum => -11017,
            RpcErrorMessage::CannotFindAddressByH160 => -11018,

            RpcErrorMessage::CannotFindSpentTransaction => -10090,

            RpcErrorMessage::AdjustAccountOnCkb => -10040,

            RpcErrorMessage::NeedAtLeastOneFromAndOneTo => -10050,
            RpcErrorMessage::RequiredCKBLessThanMin => -10051,

            RpcErrorMessage::NeedAtLeastOneFrom => -10070,
        }
    }
}
