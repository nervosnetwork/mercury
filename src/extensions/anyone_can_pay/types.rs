use crate::utils::{remove_item, to_fixed_array};

use ckb_indexer::store;
use ckb_types::{core::BlockNumber, packed, prelude::*, H160};
use derive_more::Display;
use rlp::{Decodable, DecoderError, Encodable, Prototype, Rlp, RlpStream};

use std::collections::{HashMap, HashSet};

#[derive(Debug, Display)]
pub enum ACPExtensionError {
    #[display(
        fmt = "Cannot get live cell by outpoint of tx_hash {}, index {}",
        tx_hash,
        index
    )]
    CannotGetLiveCellByOutPoint { tx_hash: String, index: u32 },

    #[display(
        fmt = "Missing ACP cell by outpoint of tx_hash {}, index {}",
        tx_hash,
        index
    )]
    MissingACPCell { tx_hash: String, index: u32 },

    #[display(fmt = "DB Error {}", _0)]
    DBError(String),
}

impl std::error::Error for ACPExtensionError {}

impl From<store::Error> for ACPExtensionError {
    fn from(err: store::Error) -> ACPExtensionError {
        ACPExtensionError::DBError(err.to_string())
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
    ACPCells(packed::OutPointVec),
    RollbackData(Vec<u8>),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::ACPCells(cells) => cells.as_bytes().to_vec(),
            Value::RollbackData(data) => data,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ACPCellList {
    pub removed: Vec<packed::OutPoint>,
    pub added: Vec<packed::OutPoint>,
}

impl ACPCellList {
    pub fn push_removed(&mut self, item: &packed::OutPoint) {
        self.removed.push(item.clone());
    }

    pub fn push_added(&mut self, item: packed::OutPoint) {
        self.added.push(item)
    }

    pub fn reverse(&mut self) {
        let tmp = self.removed.clone();
        self.removed = self.added.clone();
        self.added = tmp;
    }

    pub fn remove_intersection(&mut self) {
        let mut intersection_list = Vec::new();
        let set = self.added.iter().map(Clone::clone).collect::<HashSet<_>>();

        for item in self.removed.iter() {
            if set.contains(item) {
                remove_list.push(item.clone());
            }
        }

        for item in intersection_list.iter() {
            remove_item(&mut self.added, item);
            remove_item(&mut self.removed, item);
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let removed: packed::OutPointVec = self.removed.clone().pack();
        let added: packed::OutPointVec = self.added.clone().pack();

        let mut ret = Vec::from((removed.as_slice().len() as u32).to_be_bytes());
        ret.extend_from_slice(removed.as_slice());
        ret.extend_from_slice(added.as_slice());
        ret
    }

    #[cfg(test)]
    fn new(added: Vec<packed::OutPoint>, removed: Vec<packed::OutPoint>) -> Self {
        ACPCellList { removed, added }
    }
}

impl From<Vec<u8>> for ACPCellList {
    fn from(v: Vec<u8>) -> Self {
        let removed_len = u32::from_be_bytes(to_fixed_array(&v[0..4])) as usize;
        let removed = packed::OutPointVec::from_slice(&v[4..(4 + removed_len)]).unwrap();
        let added = packed::OutPointVec::from_slice(&v[(4 + removed_len)..]).unwrap();

        ACPCellList {
            added: added.into_iter().collect(),
            removed: removed.into_iter().collect(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ACPMap(pub HashMap<H160, ACPCellList>);

impl Encodable for ACPMap {
    fn rlp_append(&self, s: &mut RlpStream) {
        let len = self.0.len() * 2 + 1;
        s.begin_list(len).append(&len);

        for (k, v) in self.0.iter() {
            let key = k.as_bytes().to_vec();
            s.append(&key).append(&v.as_bytes());
        }
    }
}

impl Decodable for ACPMap {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.prototype()? {
            Prototype::List(_) => {
                let len: usize = rlp.val_at(0)?;
                let mut map = HashMap::new();

                for i in (1..len).step_by(2) {
                    let tmp: Vec<u8> = rlp.val_at(i)?;
                    let addr = H160::from_slice(&tmp).unwrap();
                    let tmp: Vec<u8> = rlp.val_at(i + 1)?;
                    let cell_list = ACPCellList::from(tmp);
                    map.insert(addr, cell_list);
                }

                Ok(ACPMap::new(map))
            }

            _ => Err(DecoderError::Custom("invalid prototype")),
        }
    }
}

impl ACPMap {
    fn new(map: HashMap<H160, ACPCellList>) -> Self {
        ACPMap(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::random;
    use std::collections::HashSet;
    use std::{fmt::Debug, hash::Hash};

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect()
    }

    fn mock_byte32() -> packed::Byte32 {
        let mut ret = [0u8; 32];
        ret.iter_mut().for_each(|b| *b = random::<u8>());
        ret.pack()
    }

    fn mock_h160() -> H160 {
        H160::from_slice(&rand_bytes(20)).unwrap()
    }

    fn mock_outpoint() -> packed::OutPoint {
        packed::OutPointBuilder::default()
            .tx_hash(mock_byte32())
            .index(random::<u32>().pack())
            .build()
    }

    fn assert_vec<T: Debug + Eq + Hash>(a: Vec<T>, b: Vec<T>) {
        assert_eq!(a.len(), b.len());

        let set_a = a.into_iter().collect::<HashSet<_>>();
        let set_b = b.into_iter().collect::<HashSet<_>>();

        assert_eq!(set_a, set_b);
    }

    #[test]
    fn test_acp_map_codec() {
        let mut acp_map = ACPMap::default();
        let addr_0 = mock_h160();
        let added_0 = vec![mock_outpoint(), mock_outpoint(), mock_outpoint()];
        let removed_0 = vec![mock_outpoint(), mock_outpoint()];

        let addr_1 = mock_h160();
        let added_1 = vec![mock_outpoint(), mock_outpoint()];
        let removed_1 = vec![mock_outpoint(), mock_outpoint(), mock_outpoint()];

        acp_map.0.insert(
            addr_0.clone(),
            ACPCellList::new(added_0.clone(), removed_0.clone()),
        );
        acp_map.0.insert(
            addr_1.clone(),
            ACPCellList::new(added_1.clone(), removed_1.clone()),
        );

        let bytes = rlp::encode(&acp_map);
        let decoded = rlp::decode::<ACPMap>(&bytes).unwrap();

        let decoded_0 = decoded.0.get(&addr_0).cloned().unwrap();
        let decoded_1 = decoded.0.get(&addr_1).cloned().unwrap();

        let tmp = decoded_0.added.into_iter().collect::<Vec<_>>();
        assert_eq!(added_0, tmp);

        let tmp = decoded_0.removed.into_iter().collect::<Vec<_>>();
        assert_eq!(removed_0, tmp);

        let tmp = decoded_1.added.into_iter().collect::<Vec<_>>();
        assert_eq!(added_1, tmp);

        let tmp = decoded_1.removed.into_iter().collect::<Vec<_>>();
        assert_eq!(removed_1, tmp);
    }

    #[test]
    fn test_acp_list_into_vec() {
        let added = vec![mock_outpoint(), mock_outpoint(), mock_outpoint()];
        let removed = vec![mock_outpoint(), mock_outpoint()];
        let acp_list = ACPCellList::new(added.clone(), removed.clone());

        let bytes = acp_list.as_bytes();
        let acp_list_new = ACPCellList::from(bytes);

        let added_new = acp_list_new.added.into_iter().collect::<Vec<_>>();
        let removed_new = acp_list_new.removed.into_iter().collect::<Vec<_>>();
        assert_vec(added, added_new);
        assert_vec(removed, removed_new);
    }

    #[test]
    fn test_remove_intersection() {
        let dup = mock_outpoint();
        let added = vec![dup.clone(), mock_outpoint(), mock_outpoint()];
        let removed = vec![dup.clone(), mock_outpoint()];

        let mut acp_list = ACPCellList::new(added, removed);
        acp_list.remove_intersection();

        assert!(acp_list.added.len() == 2);
        assert!(acp_list.removed.len() == 1);
        assert_eq!(acp_list.added.contains(&dup), false);
        assert_eq!(acp_list.removed.contains(&dup), false);
    }
}
