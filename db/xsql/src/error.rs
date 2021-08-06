use common::derive_more::Display;

#[derive(Clone, Debug, Display)]
pub enum DBError {
    #[display(fmt = "Get block hash and number are both None")]
    InvalidGetBlockRequest,
}

impl std::error::Error for DBError {}
