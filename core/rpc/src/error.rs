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

    #[display(fmt = "Missing {} script info", _0)]
    MissingScriptInfo(String),

    #[display(fmt = "Invalid script hash {}", _0)]
    InvalidScriptHash(String),

    #[display(fmt = "Parse address error {}", _0)]
    ParseAddressError(String),

    #[display(fmt = "Get none block from code")]
    GetNoneBlockFromNode,

    #[display(fmt = "Cannot get script by script hash")]
    CannotGetScriptByHash,

    #[display(fmt = "DB error {}", _0)]
    DBError(String),

    #[display(fmt = "Common error {}", _0)]
    CommonError(String),

    #[display(fmt = "Unsupport lock script, code hash {}", _0)]
    UnsupportLockScript(String),

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

    #[display(fmt = "Need at least one item in from")]
    NeedAtLeastOneFrom,

    #[display(fmt = "Can not find change cell")]
    CannotFindChangeCell,

    #[display(fmt = "Can not find available dao deposit cell")]
    CannotFindDepositCell,

    #[display(fmt = "Can not find available dao withdrawing cell")]
    CannotFindWithdrawingCell,

    #[display(fmt = "Cannot find transaction by hash")]
    CannotFindTransactionByHash,

    #[display(fmt = "Cannot find detailed cell by out point")]
    CannotFindDetailedCellByOutPoint,

    #[display(fmt = "Need at least one item in from and in to")]
    NeedAtLeastOneFromAndOneTo,

    #[display(fmt = "Exceed the maximum item number")]
    ExceedMaxItemNum,

    #[display(fmt = "Required CKB is less than mininum")]
    RequiredCKBLessThanMin,

    #[display(fmt = "Cannot find address by H160")]
    CannotFindAddressByH160,

    #[display(fmt = "Missing consumed Info")]
    MissingConsumedInfo,

    #[display(fmt = "Invalid DAO capacity")]
    InvalidDAOCapacity,

    #[display(fmt = "Required UDT is not enough")]
    UDTIsNotEnough,

    #[display(fmt = "Cannot find ACP cell")]
    CannotFindACPCell,

    #[display(fmt = "Transfer amount should be positive")]
    TransferAmountMustPositive,

    #[display(fmt = "Invalid adjust account number")]
    InvalidAdjustAccountNumber,
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
            RpcErrorMessage::UnsupportLockScript(_) => -11007,
            RpcErrorMessage::DecodeHexError(_) => -11008,
            RpcErrorMessage::TokenIsNotEnough(_) => -11009,
            RpcErrorMessage::OccupiedCapacityError(_) => -11010,
            RpcErrorMessage::GetEpochFromNumberError(_) => -11011,
            RpcErrorMessage::CannotFindChangeCell => -11013,
            RpcErrorMessage::CannotFindTransactionByHash => -11014,
            RpcErrorMessage::CannotFindDetailedCellByOutPoint => -11015,
            RpcErrorMessage::ExceedMaxItemNum => -11017,
            RpcErrorMessage::CannotFindAddressByH160 => -11018,

            RpcErrorMessage::MissingScriptInfo(_) => -11020,
            RpcErrorMessage::InvalidScriptHash(_) => -11021,
            RpcErrorMessage::ParseAddressError(_) => -11022,

            RpcErrorMessage::MissingConsumedInfo => -11020,

            RpcErrorMessage::CannotFindSpentTransaction => -10090,

            RpcErrorMessage::AdjustAccountOnCkb => -10040,
            RpcErrorMessage::InvalidAdjustAccountNumber => -10041,

            RpcErrorMessage::NeedAtLeastOneFromAndOneTo => -10050,
            RpcErrorMessage::RequiredCKBLessThanMin => -10051,
            RpcErrorMessage::CannotFindACPCell => -10052,
            RpcErrorMessage::TransferAmountMustPositive => -10053,

            RpcErrorMessage::UDTIsNotEnough => -10060,

            RpcErrorMessage::NeedAtLeastOneFrom => -10070,
            RpcErrorMessage::InvalidDAOCapacity => -10071,
            RpcErrorMessage::CannotFindDepositCell => -11072,

            RpcErrorMessage::CannotFindWithdrawingCell => -10110,
        }
    }
}
