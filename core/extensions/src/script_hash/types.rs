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
    TxHash = 8,
}

#[derive(Clone, Debug)]
pub enum Key {
    ScriptHash([u8; 20]),
    TxHash([u8; 32]),
}

impl Into<Vec<u8>> for Key {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();
        match self {
            Key::ScriptHash(hash) => {
                encoded.push(KeyPrefix::ScriptHash as u8);
                encoded.extend_from_slice(&hash);
            }
            Key::TxHash(hash) => {
                encoded.push(KeyPrefix::TxHash as u8);
                encoded.extend_from_slice(&hash);
            }
        }
        encoded
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
    BlockNumAndHash(u64, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Value<'a> {
    fn into(self) -> Vec<u8> {
        match self {
            Value::Script(script) => script.as_slice().to_vec(),
            Value::BlockNumAndHash(num, hash) => {
                let mut ret = num.to_be_bytes().to_vec();
                ret.extend_from_slice(&hash.raw_data());
                ret
            }
        }
    }
}
