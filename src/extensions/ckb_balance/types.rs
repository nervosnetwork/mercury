use ckb_types::bytes::Bytes;
use ckb_types::{packed, prelude::Entity};
use derive_more::Display;

#[derive(Debug, Display)]
pub enum CkbBalanceExtensionError {
    #[display(fmt = "Ckb balance is negative {:?}, address {:?}", _1, _0)]
    BalanceIsNegative(String, i128),
}

impl std::error::Error for CkbBalanceExtensionError {}

#[repr(u8)]
pub enum KeyPrefix {
    CkbBalance = 254,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    CkbAddress(&'a Bytes),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::CkbAddress(key) => {
                encoded.push(KeyPrefix::CkbBalance as u8);
                encoded.extend_from_slice(key.as_ref());
            }
        }

        encoded
    }
}

pub enum Value {
    CkbBalance(u64),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::CkbBalance(balance) => Vec::from(balance.to_le_bytes()),
        }
    }
}
