use ckb_indexer::store::Error as StoreError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("DB error: {0}")]
    DBError(String),
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => Error::DBError(s),
        }
    }
}
