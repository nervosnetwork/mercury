use crate::extensions::to_fixed_array;

use ckb_types::bytes::Bytes;
use ckb_types::{core::BlockNumber, packed, prelude::Entity};
use derive_more::Display;
use rlp::{Decodable, DecoderError, Encodable, Prototype, Rlp, RlpStream};

use std::collections::HashMap;

#[derive(Debug, Display)]
pub enum CkbBalanceExtensionError {
    #[display(fmt = "Ckb balance is negative {:?}, address {:?}", _1, _0)]
    BalanceIsNegative(String, i128),
}

impl std::error::Error for CkbBalanceExtensionError {}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 0,
    Block = 16,
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    CkbAddress(&'a Bytes),
    Block(BlockNumber, &'a packed::Byte32),
}

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::CkbAddress(key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(key.as_ref());
            }

            Key::Block(block_num, block_hash) => {
                encoded.push(KeyPrefix::Block as u8);
                encoded.extend_from_slice(&block_num.to_le_bytes());
                encoded.extend_from_slice(block_hash.as_slice());
            }
        }

        encoded
    }
}

pub enum Value {
    CkbBalance(u64),
    RollbackData(Bytes),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        match self {
            Value::CkbBalance(balance) => Vec::from(balance.to_le_bytes()),
            Value::RollbackData(data) => data.to_vec(),
        }
    }
}

struct DeltaBalance {
    addr:    Bytes,
    balance: i128,
}

impl DeltaBalance {
    fn new(addr: Bytes, balance: i128) -> Self {
        DeltaBalance { addr, balance }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::from(self.balance.to_le_bytes());
        ret.extend_from_slice(&self.addr.to_vec());
        ret
    }
}

impl From<Vec<u8>> for DeltaBalance {
    fn from(v: Vec<u8>) -> Self {
        let balance = i128::from_le_bytes(to_fixed_array(&v[0..16]));
        let addr = Bytes::from(v[16..].to_vec());
        DeltaBalance { addr, balance }
    }
}

#[derive(Default, Clone, Debug)]
pub struct CkbBalanceMap(HashMap<Bytes, i128>);

impl Encodable for CkbBalanceMap {
    fn rlp_append(&self, s: &mut RlpStream) {
        let len = self.len();

        s.begin_list(len + 1);
        s.append(&len);

        self.0.iter().for_each(|(k, v)| {
            let delta = DeltaBalance::new(k.clone(), *v);
            s.append(&delta.as_bytes());
        });
    }
}

impl Decodable for CkbBalanceMap {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.prototype()? {
            Prototype::List(_) => {
                let len: usize = rlp.val_at(0)?;
                let mut map = HashMap::new();

                for i in 0..(len + 1) {
                    let bytes: Vec<u8> = rlp.val_at(i)?;
                    let delta = DeltaBalance::from(bytes);
                    map.insert(delta.addr, delta.balance);
                }

                Ok(CkbBalanceMap::new(map))
            }

            _ => return Err(DecoderError::Custom("invalid prototype")),
        }
    }
}

impl CkbBalanceMap {
    pub fn new(map: HashMap<Bytes, i128>) -> Self {
        CkbBalanceMap(map)
    }

    pub fn inner(&self) -> &HashMap<Bytes, i128> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut HashMap<Bytes, i128> {
        &mut self.0
    }

    pub fn take(self) -> HashMap<Bytes, i128> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
