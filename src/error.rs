use ckb_indexer::store::Error as StoreError;
use derive_more::Display;

#[allow(dead_code)]
#[derive(Debug, Display)]
pub enum MercuryError {
    #[display(fmt = "DB error: {:?}", _0)]
    DBError(String),

    #[display(fmt = "Parse CKB address error {:?}", _0)]
    ParseCKBAddressError(String),

    #[display(fmt = "Already a short CKB address")]
    AlreadyShortCKBAddress,
}

impl std::error::Error for MercuryError {}

impl From<StoreError> for MercuryError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => MercuryError::DBError(s),
        }
    }
}
