use ckb_indexer::store::Error as StoreError;
use derive_more::Display;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Display)]
pub enum MercuryError {
    #[display(fmt = "DB error: {:?}", _0)]
    DBError(String),
}

impl std::error::Error for MercuryError {}

impl From<StoreError> for MercuryError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => MercuryError::DBError(s),
        }
    }
}
