use common::derive_more::Display;

use ckb_indexer::store;
use ckb_types::{packed, prelude::Entity};

#[derive(Debug, Display)]
pub enum ScriptHashExtensionError {
    #[display(fmt = "DB Error {}", _0)]
    DBError(String),
}

impl std::error::Error for ScriptHashExtensionError {}

impl From<store::Error> for ScriptHashExtensionError {
    fn from(err: store::Error) -> Self {
        ScriptHashExtensionError::DBError(err.to_string())
    }
}

#[repr(u8)]
pub enum KeyPrefix {
    ScriptHash = 0,
}

#[derive(Clone, Debug)]
pub enum Key {
    ScriptHash([u8; 20]),
}

impl Into<Vec<u8>> for Key {
    fn into(self) -> Vec<u8> {
        match self {
            Key::ScriptHash(hash) => {
                let mut encoded = vec![KeyPrefix::ScriptHash as u8];
                encoded.extend_from_slice(&hash);
                encoded
            }
        }
    }
}

impl Key {
    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}

#[derive(Clone, Debug)]
pub enum Value<'a> {
    Script(&'a packed::Script),
}

impl<'a> Into<Vec<u8>> for Value<'a> {
    fn into(self) -> Vec<u8> {
        match self {
            Value::Script(script) => script.as_slice().to_vec(),
        }
    }
}
