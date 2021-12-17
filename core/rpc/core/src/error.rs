use common::derive_more::Display;
use core_rpc_types::error::{MercuryRpcError, RpcError};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum CoreError {
    #[display(fmt = "Missing {} script info", _0)]
    MissingScriptInfo(String),

    #[display(fmt = "Invalid script hash {}", _0)]
    InvalidScriptHash(String),

    #[display(fmt = "Parse address error {}", _0)]
    ParseAddressError(String),

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

    #[display(fmt = "Can not find unlocked dao withdrawing cell")]
    CannotFindUnlockedWithdrawingCell,

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

    #[display(fmt = "Required UDT is not enough: {}", _0)]
    UDTIsNotEnough(String),

    #[display(fmt = "Cannot find ACP cell")]
    CannotFindACPCell,

    #[display(fmt = "Transfer amount should be positive")]
    TransferAmountMustPositive,

    #[display(fmt = "Invalid adjust account number")]
    InvalidAdjustAccountNumber,

    #[display(fmt = "Input UDT amount should be 0")]
    NotZeroInputUDTAmount,

    #[display(fmt = "Invalid outpoint")]
    InvalidOutPoint,

    #[display(fmt = "Overflow")]
    Overflow,

    #[display(fmt = "The input items must be the same kind of enumeration")]
    ItemsNotSameEnumValue,

    #[display(fmt = "Need at least one item in to")]
    NeedAtLeastOneTo,

    #[display(fmt = "Unsupport identity flag")]
    UnsupportIdentityFlag,

    #[display(fmt = "Unsupport ownership")]
    UnsupportOwnership,

    #[display(fmt = "Unsupport address")]
    UnsupportAddress,

    #[display(fmt = "Invalid fee change")]
    InvalidFeeChange,

    #[display(fmt = "Invalid tx prebuilt {}", _0)]
    InvalidTxPrebuilt(String),

    #[display(fmt = "From items must not contain the to item")]
    FromContainTo,

    #[display(fmt = "Ckb client error {}", _0)]
    CkbClientError(String),

    #[display(fmt = "Required CKB is not enough: {}", _0)]
    CkbIsNotEnough(String),
}

impl RpcError for CoreError {
    fn err_code(&self) -> i32 {
        match self {
            CoreError::InvalidRpcParams(_) => -11002,
            CoreError::GetNoneBlockFromNode => -11003,
            CoreError::CannotGetScriptByHash => -11004,
            CoreError::DBError(_) => -11005,
            CoreError::CommonError(_) => -11006,
            CoreError::UnsupportLockScript(_) => -11007,
            CoreError::DecodeHexError(_) => -11008,
            CoreError::TokenIsNotEnough(_) => -11009,
            CoreError::OccupiedCapacityError(_) => -11010,
            CoreError::GetEpochFromNumberError(_) => -11011,
            CoreError::CannotFindChangeCell => -11013,
            CoreError::CannotFindTransactionByHash => -11014,
            CoreError::CannotFindDetailedCellByOutPoint => -11015,
            CoreError::ExceedMaxItemNum => -11017,
            CoreError::CannotFindAddressByH160 => -11018,
            CoreError::Overflow => -11019,
            CoreError::MissingScriptInfo(_) => -11020,
            CoreError::InvalidScriptHash(_) => -11021,
            CoreError::ParseAddressError(_) => -11022,
            CoreError::ItemsNotSameEnumValue => -11023,
            CoreError::UnsupportIdentityFlag => -11024,
            CoreError::UnsupportOwnership => -11025,
            CoreError::UnsupportAddress => -11026,
            CoreError::InvalidTxPrebuilt(_) => -11027,
            CoreError::CkbClientError(_) => -11028,
            CoreError::CkbIsNotEnough(_) => -11029,
            CoreError::UDTIsNotEnough(_) => -11030,

            CoreError::MissingConsumedInfo => -10020,

            CoreError::CannotFindSpentTransaction => -10090,

            CoreError::AdjustAccountOnCkb => -10040,
            CoreError::InvalidAdjustAccountNumber => -10041,
            CoreError::NotZeroInputUDTAmount => -10042,

            CoreError::NeedAtLeastOneFromAndOneTo => -10050,
            CoreError::RequiredCKBLessThanMin => -10051,
            CoreError::CannotFindACPCell => -10052,
            CoreError::TransferAmountMustPositive => -10053,
            CoreError::InvalidFeeChange => -10054,
            CoreError::FromContainTo => -10055,

            CoreError::NeedAtLeastOneFrom => -10070,
            CoreError::InvalidDAOCapacity => -10071,
            CoreError::CannotFindDepositCell => -11072,

            CoreError::CannotFindUnlockedWithdrawingCell => -10110,
            CoreError::InvalidOutPoint => -10111,

            CoreError::NeedAtLeastOneTo => -10120,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<CoreError> for MercuryRpcError {
    fn from(err: CoreError) -> Self {
        MercuryRpcError(Box::new(err))
    }
}
