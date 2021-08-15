use common::derive_more::Display;

#[derive(Clone, Debug, Display)]
pub enum DBError {
    #[display(fmt = "Get block hash and number are both None")]
    InvalidGetBlockRequest,
    #[display(fmt = "The block number mismatches block hash")]
    MismatchBlockHash,
}

impl std::error::Error for DBError {}
