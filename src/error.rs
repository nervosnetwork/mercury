use ckb_indexer::store::Error as StoreError;
use derive_more::Display;

#[derive(Debug, Display)]
pub enum Error {
    #[display(fmt = "DB error: {:?}", _0)]
    DBError(String),
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DBError(s) => Error::DBError(s),
        }
    }
}
