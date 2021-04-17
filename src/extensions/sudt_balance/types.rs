use crate::extensions::to_fixed_array;

use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::Entity};
use num_bigint::BigInt;
use rlp::{Decodable, DecoderError, Encodable, Prototype, Rlp, RlpStream};

use std::collections::HashMap;

pub enum Key<'a> {
    Address(&'a Bytes, &'a packed::Byte32),
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
            Key::Address(script_args, key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(&script_args);
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

pub enum Value {
    SUDTBalacne(u128),
    RollbackData(Bytes),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::SUDTBalacne(balance) => Vec::from(balance.to_be_bytes()),
            Value::RollbackData(data) => data.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SUDTDeltaBalance {
    key: Vec<u8>,
    balance: BigInt,
}

impl From<Vec<u8>> for SUDTDeltaBalance {
    fn from(v: Vec<u8>) -> Self {
        let len = u32::from_le_bytes(to_fixed_array(&v[0..4])) as usize;
        let key = Vec::from(&v[4..len + 4]);
        let balance = BigInt::from_signed_bytes_le(&v[4 + len..]);
        SUDTDeltaBalance { key, balance }
    }
}

impl SUDTDeltaBalance {
    fn new(key: Vec<u8>, balance: BigInt) -> Self {
        SUDTDeltaBalance { key, balance }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let len = self.key.len() as u32;
        let mut ret = len.to_le_bytes().to_vec();
        ret.extend_from_slice(&self.key);
        ret.extend_from_slice(&self.balance.to_signed_bytes_le());
        ret
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct SUDTBalanceMap(HashMap<Vec<u8>, BigInt>);

impl Encodable for SUDTBalanceMap {
    fn rlp_append(&self, s: &mut RlpStream) {
        let len = self.len();
        s.begin_list(len + 1);
        s.append(&len);

        self.inner().iter().for_each(|(k, v)| {
            let delta = SUDTDeltaBalance::new(k.clone(), v.clone());
            s.append(&delta.as_bytes());
        });
    }
}

impl Decodable for SUDTBalanceMap {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.prototype()? {
            Prototype::List(_) => {
                let len: usize = rlp.val_at(0)?;
                let mut map = HashMap::new();

                for i in 1..(len + 1) {
                    let bytes: Vec<u8> = rlp.val_at(i)?;
                    let delta = SUDTDeltaBalance::from(bytes);
                    map.insert(delta.key, delta.balance);
                }

                Ok(SUDTBalanceMap::new(map))
            }

            _ => Err(DecoderError::Custom("invalid prototype")),
        }
    }
}

impl SUDTBalanceMap {
    pub fn new(map: HashMap<Vec<u8>, BigInt>) -> Self {
        SUDTBalanceMap(map)
    }

    pub fn inner(&self) -> &HashMap<Vec<u8>, BigInt> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut HashMap<Vec<u8>, BigInt> {
        &mut self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn opposite_value(self) -> Self {
        let new_map = self
            .take()
            .into_iter()
            .map(|(k, v)| {
                let new_val: BigInt = v * -1;
                (k, new_val)
            })
            .collect::<HashMap<_, _>>();

        SUDTBalanceMap::new(new_map)
    }

    pub fn take(self) -> HashMap<Vec<u8>, BigInt> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
    }

    #[test]
    fn test_sudt_delta_balance_codec() {
        for _i in 0..10 {
            let key = rand_bytes(32);
            let balance = BigInt::from(random::<i128>());

            let delta = SUDTDeltaBalance::new(key, balance);
            let bytes = delta.as_bytes();

            assert_eq!(delta, SUDTDeltaBalance::from(bytes));
        }
    }

    #[test]
    fn test_sudt_balance_map_codec() {
        for _i in 0..10 {
            let key_1 = rand_bytes(32);
            let val_1 = BigInt::from(random::<i128>());
            let key_2 = rand_bytes(32);
            let val_2 = BigInt::from(random::<i128>());

            let mut origin_map = SUDTBalanceMap::default();
            let map = origin_map.inner_mut();

            map.insert(key_1.clone(), val_1.clone());
            map.insert(key_2.clone(), val_2.clone());

            let bytes = origin_map.rlp_bytes();
            assert_eq!(origin_map, rlp::decode::<SUDTBalanceMap>(&bytes).unwrap());
        }
    }

    #[test]
    fn test_sudt_balance_map() {
        let key_1 = rand_bytes(32);
        let val_1 = BigInt::from(random::<i128>());
        let key_2 = rand_bytes(32);
        let val_2 = BigInt::from(random::<i128>());

        let mut origin_map = SUDTBalanceMap::default();
        let map = origin_map.inner_mut();

        map.insert(key_1.clone(), val_1.clone());
        map.insert(key_2.clone(), val_2.clone());

        let origin_map = origin_map.opposite_value();
        let origin_clone = origin_map.clone();
        let map = origin_clone.inner();
        let map_clone = map.clone();

        assert_eq!(origin_map.len(), 2);
        assert_eq!(origin_map.take(), map_clone);
        assert_eq!(*map.get(&key_1).unwrap(), (0 - val_1));
        assert_eq!(*map.get(&key_2).unwrap(), (0 - val_2));
    }
}
