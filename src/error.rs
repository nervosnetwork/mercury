use ckb_indexer::store::Error as StoreError;
use derive_more::Display;

#[allow(dead_code)]
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

    #[display(fmt = "UDT min is some when Ckb min is none")]
    InvalidAccountUDTMin,

    #[display(
        fmt = "Cannot get live cell by outpoint tx_hash {}, index {}",
        tx_hash,
        index
    )]
    CannotGetLiveCellByOutPoint { tx_hash: String, index: u32 },
}

impl std::error::Error for MercuryError {}

impl From<StoreError> for MercuryError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => MercuryError::DBError(s),
        }
    }
}
