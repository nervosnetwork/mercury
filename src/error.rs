use ckb_indexer::store::Error as StoreError;
use ckb_jsonrpc_types::OutPoint;
use derive_more::Display;
use smt::error::Error as SMTError;

#[derive(Debug, Display)]
pub enum MercuryError {
    #[display(fmt = "DB error: {}", _0)]
    DBError(String),

    #[display(fmt = "Parse CKB address error {}", _0)]
    ParseCKBAddressError(String),

    #[display(fmt = "Already a short CKB address")]
    AlreadyShortCKBAddress,

    #[display(fmt = "UDT {} is inexistent", _0)]
    UDTInexistence(String),

    #[display(fmt = "The address {} has no acp cell", _0)]
    NoACPInThisAddress(String),

    #[display(fmt = "Lack of ACP to pay for udt capacity, address {}", _0)]
    LackACPCells(String),

    #[display(fmt = "Missing ACP cell with type_hash {}, address {}", _1, _0)]
    MissingACPCell(String, String),

    #[display(fmt = "Lack of sUDT cell of address{}", _0)]
    LackSUDTCells(String),

    #[display(fmt = "Ckb is not enough, address {}", _0)]
    CkbIsNotEnough(String),

    #[display(fmt = "UDT is not enough, address {}", _0)]
    UDTIsNotEnough(String),

    #[display(fmt = "UDT min is some when Ckb min is none")]
    InvalidAccountUDTMin,

    #[display(fmt = "Invalid create account info")]
    InvalidAccountInfo,

    #[display(fmt = "Ckb transfer can only pay by from")]
    InvalidTransferPayload,

    #[display(fmt = "Missing config of {:?} script", _0)]
    MissingConfig(String),

    #[display(
        fmt = "Cannot get live cell by outpoint tx_hash {}, index {}",
        tx_hash,
        index
    )]
    CannotGetLiveCellByOutPoint {
        tx_hash: String,
        index: u32,
    },
    _AlreadyShortCKBAddress,

    #[display(fmt = "Cannot find cell by out point {:?}", _0)]
    CannotFindCellByOutPoint(OutPoint),

    #[display(fmt = "Sparse merkle tree error {:?}", _0)]
    SMTError(String),

    #[display(fmt = "Output must be rce cell when update rce rule")]
    InvalidOutputCellWhenUpdateRCE,

    #[display(fmt = "Missing RC data")]
    MissingRCData,

    #[display(fmt = "The rce rule number {} is above 8196", _0)]
    RCRuleNumOverMax(usize),

    #[display(fmt = "Check white list failed, script hash {:?}", _0)]
    CheckWhiteListFailed(String),

    #[display(fmt = "Check black list failed, script hash {:?}", _0)]
    CheckBlackListFailed(String),

    #[display(fmt = "Rce rule is in stop state, root {:?}", _0)]
    RCRuleIsInStopState(String),
}

impl std::error::Error for MercuryError {}

impl From<StoreError> for MercuryError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => MercuryError::DBError(s),
        }
    }
}

impl From<SMTError> for MercuryError {
    fn from(error: SMTError) -> Self {
        MercuryError::SMTError(error.to_string())
    }
}
