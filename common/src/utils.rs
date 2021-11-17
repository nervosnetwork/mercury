use crate::MercuryError;

use crate::address::{Address, AddressPayload, CodeHashIndex};
use crate::NetworkType;

use anyhow::Result;
use ckb_types::{packed, H160, U256};
use derive_more::Display;
use num_bigint::BigUint;

use std::convert::TryInto;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInfo {
    pub script: packed::Script,
    pub cell_dep: packed::CellDep,
}

#[derive(Clone, Debug, Display)]
enum UtilsError {
    #[display(fmt = "Already a short CKB address")]
    _AlreadyShortCKBAddress,

    #[display(fmt = "Parse CKB address error {}", _0)]
    ParseCKBAddressError(String),
}

// Convert to short address, use as universal identity
pub fn parse_address(input: &str) -> Result<Address> {
    match Address::from_str(input) {
        Ok(addr) => Ok(addr),
        Err(e) => Err(MercuryError::utils(UtilsError::ParseCKBAddressError(e)).into()),
    }
}

pub fn to_universal_identity(net_ty: NetworkType, input: &Address, is_new: bool) -> Address {
    Address::new(
        input.network(),
        AddressPayload::new_short(
            net_ty,
            CodeHashIndex::Sighash,
            H160::from_slice(input.payload().args().as_ref()).unwrap(),
        ),
        is_new,
    )
}

pub fn to_fixed_array<const LEN: usize>(input: &[u8]) -> [u8; LEN] {
    assert_eq!(input.len(), LEN);
    let mut list = [0; LEN];
    list.copy_from_slice(input);
    list
}

pub fn find<T: Eq>(key: &T, from: &[T]) -> Option<usize> {
    for (index, item) in from.iter().enumerate() {
        if item == key {
            return Some(index);
        }
    }
    None
}

pub fn remove_item<T: Eq>(list: &mut Vec<T>, key: &T) {
    let mut index = usize::MAX;
    for (idx, item) in list.iter().enumerate() {
        if item == key {
            index = idx;
            break;
        }
    }

    list.remove(index);
}

#[allow(dead_code)]
pub fn u64_sub(a: u64, b: BigUint) -> u64 {
    let b: u64 = b.try_into().unwrap();
    a.saturating_sub(b)
}

pub fn u128_sub(a: u128, b: BigUint) -> u128 {
    let b: u128 = b.try_into().unwrap();
    a.saturating_sub(b)
}

pub fn unwrap_only_one<T: Clone>(vec: &[T]) -> T {
    assert!(vec.len() == 1);
    vec[0].clone()
}

pub fn decode_udt_amount(data: &[u8]) -> u128 {
    if data.len() < 16 {
        return 0u128;
    }
    u128::from_le_bytes(to_fixed_array(&data[0..16]))
}

pub fn encode_udt_amount(amount: u128) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

pub fn decode_nonce(data: &[u8]) -> u128 {
    u128::from_be_bytes(to_fixed_array(&data[0..16]))
}

pub fn decode_dao_block_number(data: &[u8]) -> u64 {
    u64::from_le_bytes(to_fixed_array(&data[0..8]))
}

pub fn decode_u64(data: &[u8]) -> u64 {
    u64::from_le_bytes(to_fixed_array(&data[0..8]))
}

pub fn u256_low_u64(u: U256) -> u64 {
    u.0[0]
}

#[cfg(test)]
mod test {
    use super::*;

    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
    }

    #[test]
    fn test_to_fixed_array() {
        let bytes = rand_bytes(3);
        let a = to_fixed_array::<3>(&bytes);
        let mut b = [0u8; 3];
        b.copy_from_slice(&bytes);

        assert_eq!(a, b);
    }

    #[test]
    fn test_find() {
        let test = (0..10).collect::<Vec<_>>();
        test.iter()
            .for_each(|i| assert_eq!(find(i, &test), Some(*i)));
    }
}
