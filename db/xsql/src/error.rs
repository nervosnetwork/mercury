use common::derive_more::Display;

#[derive(Clone, Debug, Display)]
pub enum DBError {
    #[display(fmt = "The block number mismatches block hash")]
    MismatchBlockHash,
    #[display(fmt = "The block number is wrong height")]
    WrongHeight,
    #[display(fmt = "No block with the hash was found")]
    CannotFind,
}

impl std::error::Error for DBError {}
