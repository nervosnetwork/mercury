use common::derive_more::Display;

use bincode::serialize;
use ckb_indexer::store;
use ckb_types::{core::BlockNumber, packed, prelude::Entity};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Debug, Display)]
pub enum CkbBalanceExtensionError {
    #[display(fmt = "Ckb balance is negative {:?}, address {}", _1, _0)]
    BalanceIsNegative(String, Balance),

    #[display(
        fmt = "Cannot get live cell by outpoint tx_hash {}, index {}",
        tx_hash,
        index
    )]
    NoLiveCellByOutpoint { tx_hash: String, index: u32 },

    #[display(fmt = "DB Error {}", _0)]
    DBError(String),
}

impl std::error::Error for CkbBalanceExtensionError {}

impl From<store::Error> for CkbBalanceExtensionError {
    fn from(err: store::Error) -> Self {
        CkbBalanceExtensionError::DBError(err.to_string())
    }
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    CkbAddress(&'a [u8]),
    Block(BlockNumber, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::CkbAddress(key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(key);
            }

            Key::Block(block_num, block_hash) => {
                encoded.push(KeyPrefix::Block as u8);
                encoded.extend_from_slice(&block_num.to_be_bytes());
                encoded.extend_from_slice(block_hash.as_slice());
            }
        }

        encoded
    }
}

impl<'a> Key<'a> {
    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Balance {
    pub normal_capacity: u64,
    pub udt_capacity: u64,
}

impl Balance {
    pub fn new(normal_capacity: u64, udt_capacity: u64) -> Self {
        Balance {
            normal_capacity,
            udt_capacity,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    CkbBalance(Balance),
    RollbackData(CkbBalanceMap),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::CkbBalance(balance) => serialize(&balance).unwrap(),
            Value::RollbackData(map) => serialize(&map).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct BalanceDelta {
    pub normal_capacity: i128,
    pub udt_capacity: i128,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct CkbBalanceMap(HashMap<[u8; 32], BalanceDelta>);

impl CkbBalanceMap {
    #[allow(dead_code)]
    pub fn new(map: HashMap<[u8; 32], BalanceDelta>) -> Self {
        CkbBalanceMap(map)
    }

    pub fn inner(&self) -> &HashMap<[u8; 32], BalanceDelta> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut HashMap<[u8; 32], BalanceDelta> {
        &mut self.0
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn opposite_value(&mut self) {
        self.0.iter_mut().for_each(|(_k, v)| {
            v.normal_capacity *= -1;
            v.udt_capacity *= -1;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::deserialize;
    use rand::random;

    fn rand_byte32() -> [u8; 32] {
        let mut ret = [0u8; 32];
        ret.copy_from_slice(&(0..32).map(|_| random::<u8>()).collect::<Vec<_>>());
        ret
    }

    #[test]
    fn test_ckb_balance_map_codec() {
        for _i in 0..10 {
            let key_1 = rand_byte32();
            let balance_1 = BalanceDelta {
                normal_capacity: random::<i128>(),
                udt_capacity: random::<i128>(),
            };
            let key_2 = rand_byte32();
            let balance_2 = BalanceDelta {
                normal_capacity: random::<i128>(),
                udt_capacity: random::<i128>(),
            };

            let mut origin_map = CkbBalanceMap::default();
            let map = origin_map.inner_mut();

            map.insert(key_1, balance_1);
            map.insert(key_2, balance_2);

            let bytes = serialize(&map).unwrap();
            assert_eq!(origin_map, deserialize::<CkbBalanceMap>(&bytes).unwrap());
        }
    }

    #[test]
    fn test_ckb_balance_map() {
        let key_1 = rand_byte32();
        let balance_1 = BalanceDelta {
            normal_capacity: random::<i128>(),
            udt_capacity: random::<i128>(),
        };
        let key_2 = rand_byte32();
        let balance_2 = BalanceDelta {
            normal_capacity: random::<i128>(),
            udt_capacity: random::<i128>(),
        };

        let mut origin_map = CkbBalanceMap::default();
        let map = origin_map.inner_mut();

        map.insert(key_1, balance_1);
        map.insert(key_2, balance_2);

        origin_map.opposite_value();
        let origin_clone = origin_map.clone();
        let map = origin_clone.inner();

        assert_eq!(origin_map.len(), 2);
        assert_eq!(
            map.get(&key_1).unwrap().normal_capacity,
            0 - balance_1.normal_capacity
        );
        assert_eq!(
            map.get(&key_1).unwrap().udt_capacity,
            0 - balance_1.udt_capacity
        );
        assert_eq!(
            map.get(&key_2).unwrap().normal_capacity,
            0 - balance_2.normal_capacity
        );
        assert_eq!(
            map.get(&key_2).unwrap().udt_capacity,
            0 - balance_2.udt_capacity
        );
    }
}
