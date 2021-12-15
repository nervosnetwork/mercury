use common::derive_more::Display;

#[derive(Clone, Debug, Display)]
pub enum DBError {
    #[display(fmt = "The block number mismatches block hash")]
    MismatchBlockHash,

    #[display(fmt = "The block number is wrong height")]
    WrongHeight,

    #[display(fmt = "{} not exist", _0)]
    NotExist(String),

    #[display(fmt = "Invalid parameter {}", _0)]
    InvalidParameter(String),
}

impl std::error::Error for DBError {}
