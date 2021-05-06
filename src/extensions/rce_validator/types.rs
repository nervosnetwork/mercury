use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Entity};

use std::convert::TryInto;

pub enum Key<'a> {
    Address(&'a packed::Byte32, &'a packed::Byte32),
    Block(BlockNumber, &'a packed::Byte32),
    ScriptHash(&'a packed::Byte32),
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
    ScriptHash = 32,
}

impl<'a> Key<'a> {
    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::Address(script_hash, key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(script_hash.as_slice());
                encoded.extend_from_slice(key.as_slice());
            }

            Key::Block(block_number, block_hash) => {
                encoded.push(KeyPrefix::Block as u8);
                encoded.extend_from_slice(&block_number.to_be_bytes());
                encoded.extend_from_slice(block_hash.as_slice());
            }

            Key::ScriptHash(hash) => {
                encoded.push(KeyPrefix::ScriptHash as u8);
                encoded.extend_from_slice(hash.as_slice());
            }
        }

        encoded
    }
}

pub enum Value<'a> {
    RollbackData(Vec<Bytes>, Vec<Bytes>),
    Script(&'a packed::Script),
}

impl<'a> Into<Vec<u8>> for Value<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();
        match self {
            Value::RollbackData(insertions, deletions) => {
                encoded.extend_from_slice(&(insertions.len() as u64).to_be_bytes());
                insertions.iter().for_each(|key| {
                    encoded.extend_from_slice(&(key.len() as u64).to_be_bytes());
                    encoded.extend_from_slice(&key);
                });

                encoded.extend_from_slice(&(deletions.len() as u64).to_be_bytes());
                deletions.iter().for_each(|key| {
                    encoded.extend_from_slice(&(key.len() as u64).to_be_bytes());
                    encoded.extend_from_slice(&key);
                });
            }

            Value::Script(script) => {
                encoded.extend_from_slice(script.as_slice());
            }
        }
        encoded
    }
}

impl<'a> Value<'a> {
    pub fn parse_data(slice: &[u8]) -> (Vec<Bytes>, Vec<Bytes>) {
        let mut offset = 0;
        let mut insertions = vec![];
        let insertion_count = u64::from_be_bytes(
            slice[offset..offset + 8]
                .try_into()
                .expect("insertion count"),
        ) as usize;
        offset += 8;
        for _ in 0..insertion_count {
            let len = u64::from_be_bytes(
                slice[offset..offset + 8]
                    .try_into()
                    .expect("insertion length"),
            ) as usize;
            offset += 8;
            insertions.push(Bytes::from(slice[offset..(offset + len)].to_vec()));
            offset += len;
        }
        let mut deletions = vec![];
        let deletion_count = u64::from_be_bytes(
            slice[offset..offset + 8]
                .try_into()
                .expect("deletion count"),
        ) as usize;
        offset += 8;
        for _ in 0..deletion_count {
            let len = u64::from_be_bytes(
                slice[offset..offset + 8]
                    .try_into()
                    .expect("deletion length"),
            ) as usize;
            offset += 8;
            deletions.push(Bytes::from(slice[offset..(offset + len)].to_vec()));
            offset += len;
        }
        assert!(offset == slice.len());
        (insertions, deletions)
    }
}
