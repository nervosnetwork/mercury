use crate::{DetailedCell, DetailedCells};

use common::derive_more::Display;
use common::utils::remove_item;

use bincode::serialize;
use ckb_indexer::store;
use ckb_types::{core::BlockNumber, packed, prelude::*, H160};
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

#[repr(u8)]
#[derive(Copy, Clone, Display, Debug, Hash, PartialEq, Eq)]
pub enum SpecialCellKind {
    #[display(fmt = "Anyone Can Pay cell")]
    AnyoneCanPay = 0,
    #[display(fmt = "Cheque cell")]
    ChequeDeposit,
}

#[derive(Debug, Display)]
pub enum SpecialCellsExtensionError {
    #[display(
        fmt = "Cannot get live cell by outpoint of tx_hash {}, index {}",
        tx_hash,
        index
    )]
    CannotGetLiveCellByOutPoint { tx_hash: String, index: u32 },

    #[display(
        fmt = "Missing {} Cell by outpoint of tx_hash {}, index {}",
        cell_kind,
        tx_hash,
        index
    )]
    MissingSPCell {
        cell_kind: SpecialCellKind,
        tx_hash: String,
        index: u32,
    },

    #[display(fmt = "DB Error {}", _0)]
    DBError(String),
}

impl std::error::Error for SpecialCellsExtensionError {}

impl From<store::Error> for SpecialCellsExtensionError {
    fn from(err: store::Error) -> SpecialCellsExtensionError {
        SpecialCellsExtensionError::DBError(err.to_string())
    }
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    CkbAddress(&'a H160),
    Block(BlockNumber, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::CkbAddress(key) => {
                encoded.push(KeyPrefix::Address as u8);
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
    SPCells(DetailedCells),
    RollbackData(Vec<u8>),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::SPCells(cells) => serialize(&cells).unwrap(),
            Value::RollbackData(data) => data,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SPCellList {
    pub removed: DetailedCells,
    pub added: DetailedCells,
}

impl SPCellList {
    pub fn push_removed(&mut self, item: DetailedCell) {
        self.removed.push(item);
    }

    pub fn push_added(&mut self, item: DetailedCell) {
        self.added.push(item)
    }

    pub fn reverse(&mut self) {
        let tmp = self.removed.clone();
        self.removed = self.added.clone();
        self.added = tmp;
    }

    pub fn remove_intersection(&mut self) {
        let mut intersection_list = Vec::new();
        let set = self
            .added
            .0
            .iter()
            .map(Clone::clone)
            .collect::<HashSet<_>>();

        for item in self.removed.0.iter() {
            if set.contains(item) {
                intersection_list.push(item.clone());
            }
        }

        for item in intersection_list.iter() {
            remove_item(&mut self.added.0, item);
            remove_item(&mut self.removed.0, item);
        }
    }

    #[cfg(test)]
    fn new(added: DetailedCells, removed: DetailedCells) -> Self {
        SPCellList { removed, added }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SpMap(pub HashMap<H160, SPCellList>);

impl SpMap {
    pub fn entry_and_push_add(&mut self, key: H160, add: DetailedCell) {
        self.0
            .entry(key)
            .or_insert_with(Default::default)
            .push_added(add);
    }

    pub fn entry_and_push_remove(&mut self, key: H160, remove: DetailedCell) {
        self.0
            .entry(key)
            .or_insert_with(Default::default)
            .push_removed(remove);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::mock_detailed_cell;

    use bincode::deserialize;
    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect()
    }

    fn mock_h160() -> H160 {
        H160::from_slice(&rand_bytes(20)).unwrap()
    }

    #[test]
    fn test_sp_map_codec() {
        let mut sp_map = SpMap::default();
        let addr_0 = mock_h160();
        let added_0 = DetailedCells(vec![
            mock_detailed_cell(),
            mock_detailed_cell(),
            mock_detailed_cell(),
        ]);
        let removed_0 = DetailedCells(vec![mock_detailed_cell(), mock_detailed_cell()]);

        let addr_1 = mock_h160();
        let added_1 = DetailedCells(vec![mock_detailed_cell(), mock_detailed_cell()]);
        let removed_1 = DetailedCells(vec![
            mock_detailed_cell(),
            mock_detailed_cell(),
            mock_detailed_cell(),
        ]);

        sp_map.0.insert(
            addr_0.clone(),
            SPCellList::new(added_0.clone(), removed_0.clone()),
        );
        sp_map.0.insert(
            addr_1.clone(),
            SPCellList::new(added_1.clone(), removed_1.clone()),
        );

        let bytes = serialize(&sp_map).unwrap();
        let decoded = deserialize::<SpMap>(&bytes).unwrap();

        let decoded_0 = decoded.0.get(&addr_0).cloned().unwrap();
        let decoded_1 = decoded.0.get(&addr_1).cloned().unwrap();

        assert_eq!(added_0, decoded_0.added);
        assert_eq!(removed_0, decoded_0.removed);
        assert_eq!(added_1, decoded_1.added);
        assert_eq!(removed_1, decoded_1.removed);
    }

    #[test]
    fn test_sp_list_into_vec() {
        let added = DetailedCells(vec![
            mock_detailed_cell(),
            mock_detailed_cell(),
            mock_detailed_cell(),
        ]);
        let removed = DetailedCells(vec![mock_detailed_cell(), mock_detailed_cell()]);
        let sp_list = SPCellList::new(added, removed);

        let bytes = serialize(&sp_list).unwrap();
        let sp_list_new = deserialize::<SPCellList>(&bytes).unwrap();

        assert_eq!(sp_list_new, sp_list);
    }

    #[test]
    fn test_remove_intersection() {
        let dup = mock_detailed_cell();
        let added = DetailedCells(vec![
            dup.clone(),
            mock_detailed_cell(),
            mock_detailed_cell(),
        ]);
        let removed = DetailedCells(vec![dup.clone(), mock_detailed_cell()]);

        let mut sp_list = SPCellList::new(added, removed);
        sp_list.remove_intersection();

        assert!(sp_list.added.0.len() == 2);
        assert!(sp_list.removed.0.len() == 1);
        assert_eq!(sp_list.added.0.contains(&dup), false);
        assert_eq!(sp_list.removed.0.contains(&dup), false);
    }
}
