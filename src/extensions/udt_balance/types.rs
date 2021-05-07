use crate::extensions::udt_balance::{SUDT, XUDT};
use crate::utils::to_fixed_array;

use ckb_indexer::store;
use ckb_types::{core::BlockNumber, packed, prelude::Entity};
use derive_more::Display;
use num_bigint::BigInt;
use rlp::{Decodable, DecoderError, Encodable, Prototype, Rlp, RlpStream};

use std::collections::HashMap;

#[derive(Debug, Display)]
pub enum UDTType {
    #[display(fmt = "sUDT")]
    Simple,

    #[display(fmt = "xUDT")]
    Extensible,
}

impl UDTType {
    pub fn as_str(&self) -> &str {
        match self {
            UDTType::Simple => SUDT,
            UDTType::Extensible => XUDT,
        }
    }
}

#[derive(Debug, Display)]
pub enum UDTBalanceExtensionError {
    #[display(
        fmt = "SUDT balance is negative {:?}, sudt_type_hash {:?}, address {:?}",
        balance,
        sudt_type_hash,
        user_address
    )]
    BalanceIsNegative {
        sudt_type_hash: String,
        user_address: String,
        balance: BigInt,
    },

    #[display(fmt = "DB Error {:?}", _0)]
    DBError(String),
}

impl std::error::Error for UDTBalanceExtensionError {}

impl From<store::Error> for UDTBalanceExtensionError {
    fn from(err: store::Error) -> Self {
        UDTBalanceExtensionError::DBError(err.to_string())
    }
}

pub enum Key<'a> {
    Address(&'a [u8]),
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
            Key::Address(addr) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(&addr);
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
    SUDTBalance(u128),
    RollbackData(Vec<u8>),
    Script(&'a packed::Script),
}

impl<'a> Into<Vec<u8>> for Value<'a> {
    fn into(self) -> Vec<u8> {
        match self {
            Value::SUDTBalance(balance) => Vec::from(balance.to_be_bytes()),
            Value::RollbackData(data) => data,
            Value::Script(script) => script.as_slice().to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UDTDeltaBalance {
    key: Vec<u8>,
    balance: BigInt,
}

impl From<Vec<u8>> for UDTDeltaBalance {
    fn from(v: Vec<u8>) -> Self {
        let len = u32::from_le_bytes(to_fixed_array(&v[0..4])) as usize;
        let key = Vec::from(&v[4..len + 4]);
        let balance = BigInt::from_signed_bytes_le(&v[4 + len..]);
        UDTDeltaBalance { key, balance }
    }
}

impl UDTDeltaBalance {
    fn new(key: Vec<u8>, balance: BigInt) -> Self {
        UDTDeltaBalance { key, balance }
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
pub struct UDTBalanceMap(HashMap<Vec<u8>, BigInt>);

impl Encodable for UDTBalanceMap {
    fn rlp_append(&self, s: &mut RlpStream) {
        let len = self.len();
        s.begin_list(len + 1);
        s.append(&len);

        self.inner().iter().for_each(|(k, v)| {
            let delta = UDTDeltaBalance::new(k.clone(), v.clone());
            s.append(&delta.as_bytes());
        });
    }
}

impl Decodable for UDTBalanceMap {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.prototype()? {
            Prototype::List(_) => {
                let len: usize = rlp.val_at(0)?;
                let mut map = HashMap::new();

                for i in 1..(len + 1) {
                    let bytes: Vec<u8> = rlp.val_at(i)?;
                    let delta = UDTDeltaBalance::from(bytes);
                    map.insert(delta.key, delta.balance);
                }

                Ok(UDTBalanceMap::new(map))
            }

            _ => Err(DecoderError::Custom("invalid prototype")),
        }
    }
}

impl UDTBalanceMap {
    pub fn new(map: HashMap<Vec<u8>, BigInt>) -> Self {
        UDTBalanceMap(map)
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

        UDTBalanceMap::new(new_map)
    }

    pub fn take(self) -> HashMap<Vec<u8>, BigInt> {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UDTBalanceMaps {
    pub sudt: UDTBalanceMap,
    pub xudt: UDTBalanceMap,
}

impl Encodable for UDTBalanceMaps {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2).append(&self.sudt).append(&self.xudt);
    }
}

impl Decodable for UDTBalanceMaps {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.prototype()? {
            Prototype::List(2) => {
                let sudt: UDTBalanceMap = rlp.val_at(0)?;
                let xudt: UDTBalanceMap = rlp.val_at(1)?;

                Ok(UDTBalanceMaps::new(sudt, xudt))
            }

            _ => Err(DecoderError::Custom("invalid prototype")),
        }
    }
}

impl UDTBalanceMaps {
    pub fn new(sudt: UDTBalanceMap, xudt: UDTBalanceMap) -> Self {
        UDTBalanceMaps { sudt, xudt }
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
    fn test_udt_delta_balance_codec() {
        for _i in 0..10 {
            let key = rand_bytes(32);
            let balance = BigInt::from(random::<i128>());

            let delta = UDTDeltaBalance::new(key, balance);
            let bytes = delta.as_bytes();

            assert_eq!(delta, UDTDeltaBalance::from(bytes));
        }
    }

    #[test]
    fn test_udt_balance_map_codec() {
        for _i in 0..10 {
            let key_1 = rand_bytes(32);
            let val_1 = BigInt::from(random::<i128>());
            let key_2 = rand_bytes(32);
            let val_2 = BigInt::from(random::<i128>());

            let mut origin_map = UDTBalanceMap::default();
            let map = origin_map.inner_mut();

            map.insert(key_1.clone(), val_1.clone());
            map.insert(key_2.clone(), val_2.clone());

            let bytes = origin_map.rlp_bytes();
            assert_eq!(
                origin_map,
                UDTBalanceMap::decode(&Rlp::new(&bytes)).unwrap()
            );
        }
    }

    #[test]
    fn test_sudt_balance_maps_codec() {
        for _i in 0..10 {
            let key_1 = rand_bytes(32);
            let val_1 = BigInt::from(random::<i128>());
            let key_2 = rand_bytes(32);
            let val_2 = BigInt::from(random::<i128>());

            let mut origin_map = UDTBalanceMap::default();
            let map = origin_map.inner_mut();

            map.insert(key_1.clone(), val_1.clone());
            map.insert(key_2.clone(), val_2.clone());

            let maps = UDTBalanceMaps::new(origin_map.clone(), origin_map);
            let bytes = maps.rlp_bytes();
            assert_eq!(maps, UDTBalanceMaps::decode(&Rlp::new(&bytes)).unwrap());
        }
    }

    #[test]
    fn test_udt_balance_map() {
        let key_1 = rand_bytes(32);
        let val_1 = BigInt::from(random::<i128>());
        let key_2 = rand_bytes(32);
        let val_2 = BigInt::from(random::<i128>());

        let mut origin_map = UDTBalanceMap::default();
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
