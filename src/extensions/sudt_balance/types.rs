use ckb_types::{packed, prelude::Entity};

#[repr(u8)]
pub enum KeyPrefix {
    SUDTBalacne = 253,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    SUDTAddress(&'a packed::Bytes, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::SUDTAddress(script_args, key) => {
                encoded.push(KeyPrefix::SUDTBalacne as u8);
                encoded.extend_from_slice(script_args.as_slice());
                encoded.extend_from_slice(key.as_slice());
            }
        }

        encoded
    }
}

pub enum Value {
    SUDTBalacne(u128),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::SUDTBalacne(balance) => Vec::from(balance.to_be_bytes()),
        }
    }
}
