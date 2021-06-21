pub mod utils;

pub use {anyhow, derive_more};

use derive_more::Display;

use std::fmt::{Debug, Display};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ErrorKind {
    Cli,
    Extension,
    Rpc,
    Service,
    Storage,
    Utils,
}

#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(fmt = "Mercury {:?} Error {:?}", kind, error)]
pub struct MercuryError<T> {
    kind: ErrorKind,
    error: T,
}

impl<T: Debug + Display> std::error::Error for MercuryError<T> {}

impl<T: Debug + Display> MercuryError<T> {
    pub fn cli(error: T) -> Self {
        Self::new(ErrorKind::Cli, error)
    }

    pub fn extension(error: T) -> Self {
        Self::new(ErrorKind::Extension, error)
    }

    pub fn rpc(error: T) -> Self {
        Self::new(ErrorKind::Rpc, error)
    }

    pub fn service(error: T) -> Self {
        Self::new(ErrorKind::Service, error)
    }

    pub fn storage(error: T) -> Self {
        Self::new(ErrorKind::Storage, error)
    }

    pub fn utils(error: T) -> Self {
        Self::new(ErrorKind::Utils, error)
    }

    fn new(kind: ErrorKind, error: T) -> Self {
        MercuryError { kind, error }
    }
}
