use crate::extensions::MATURE_THRESHOLD;

use ckb_indexer::store;
use ckb_types::core::{BlockNumber, Capacity, RationalU256};
use ckb_types::{packed, prelude::*};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use std::collections::VecDeque;

#[derive(Debug, Display)]
pub enum LocktimeExtensionError {
    #[display(fmt = "DB Error {}", _0)]
    DBError(String),
}

impl std::error::Error for LocktimeExtensionError {}

impl From<store::Error> for LocktimeExtensionError {
    fn from(err: store::Error) -> LocktimeExtensionError {
        LocktimeExtensionError::DBError(err.to_string())
    }
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    CkbAddress(&'a str),
    Block(BlockNumber, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::CkbAddress(key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.push(key.as_bytes().len() as u8);
                encoded.extend_from_slice(key.as_bytes());
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

#[derive(Clone, Debug)]
pub enum Value {
    CellbaseCapacity(Vec<u8>),
    RollbackData(Vec<u8>),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::CellbaseCapacity(data) => data,
            Value::RollbackData(data) => data,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct CellbaseCkbAccount {
    maturity: u64,
    immature: VecDeque<CellbaseCkb>,
}

impl CellbaseCkbAccount {
    pub fn push(&mut self, cellbase: CellbaseCkb) {
        self.immature.push_back(cellbase);
    }

    pub fn remove(&mut self, cellbase: &CellbaseCkb) {
        let mut index = usize::MAX;

        for (idx, item) in self.immature.iter().enumerate() {
            if item == cellbase {
                index = idx;
                break;
            }
        }

        if index != usize::MAX {
            self.immature.remove(index);
        }
    }

    pub fn mature(&mut self) {
        let mut mature_ckb = 0u64;
        let threshold = MATURE_THRESHOLD.read();
        while let Some(front) = self.immature.front() {
            if *threshold < front.epoch {
                let tmp = self.immature.pop_front().unwrap();
                mature_ckb += tmp.capacity.as_u64();
            } else {
                break;
            }
        }

        self.maturity += mature_ckb;
    }

    #[cfg(test)]
    fn from_vec_cellbase(vec: Vec<CellbaseCkb>) -> Self {
        CellbaseCkbAccount::new(rand::random(), vec.into_iter().collect())
    }

    #[cfg(test)]
    pub fn new(maturity: u64, immature: VecDeque<CellbaseCkb>) -> Self {
        CellbaseCkbAccount { maturity, immature }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CellbaseWithAddress {
    pub address: String,
    pub cellbase: CellbaseCkb,
}

impl CellbaseWithAddress {
    pub fn new(address: String, cellbase: CellbaseCkb) -> Self {
        CellbaseWithAddress { address, cellbase }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CellbaseCkb {
    pub epoch: RationalU256,
    pub capacity: Capacity,
}

impl CellbaseCkb {
    pub fn new(epoch: RationalU256, capacity: Capacity) -> Self {
        CellbaseCkb { epoch, capacity }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ckb_types::U256;
    use rand::random;

    fn mock_u256() -> U256 {
        U256::thread_random()
    }

    #[test]
    fn test_cellbase_ckb_codec() {
        let cellbase = CellbaseCkb::new(
            RationalU256::new(mock_u256(), mock_u256()),
            Capacity::shannons(random::<u64>()),
        );
        let bytes = bincode::serialize(&cellbase).unwrap();
        assert_eq!(
            bincode::deserialize::<CellbaseCkb>(&bytes).unwrap(),
            cellbase
        );
    }

    #[test]
    fn test_cellbase_with_address_codec() {
        let address = String::from("ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve");
        let cellbase = CellbaseCkb::new(
            RationalU256::new(mock_u256(), mock_u256()),
            Capacity::shannons(random::<u64>()),
        );
        let cellbase_addr = CellbaseWithAddress::new(address, cellbase);
        let bytes = bincode::serialize(&cellbase_addr).unwrap();
        assert_eq!(
            bincode::deserialize::<CellbaseWithAddress>(&bytes).unwrap(),
            cellbase_addr
        );
    }

    #[test]
    fn test_cellbase_account_codec() {
        let cellbases = vec![
            CellbaseCkb::new(
                RationalU256::new(mock_u256(), mock_u256()),
                Capacity::shannons(random::<u64>()),
            ),
            CellbaseCkb::new(
                RationalU256::new(mock_u256(), mock_u256()),
                Capacity::shannons(random::<u64>()),
            ),
            CellbaseCkb::new(
                RationalU256::new(mock_u256(), mock_u256()),
                Capacity::shannons(random::<u64>()),
            ),
        ];
        let account = CellbaseCkbAccount::from_vec_cellbase(cellbases);
        let bytes = bincode::serialize(&account).unwrap();
        assert_eq!(
            bincode::deserialize::<CellbaseCkbAccount>(&bytes).unwrap(),
            account
        );
    }
}
