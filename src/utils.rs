use crate::error::MercuryError;

use anyhow::Result;
use ckb_sdk::{Address, AddressPayload, AddressType, CodeHashIndex};
use ckb_types::H160;
use num_bigint::BigUint;

use std::convert::TryInto;
use std::str::FromStr;

pub fn parse_address(input: &str) -> Result<Address> {
    Address::from_str(input).map_err(|e| MercuryError::ParseCKBAddressError(e).into())
}

pub fn _to_short_address(input: &Address) -> Result<Address> {
    if input.payload().ty() == AddressType::Short {
        return Err(MercuryError::AlreadyShortCKBAddress.into());
    }

    // The input type is Address. It is impossible to panic here.
    Ok(Address::new(
        input.network(),
        AddressPayload::new_short(
            CodeHashIndex::Sighash,
            H160::from_slice(input.payload().args().as_ref()).unwrap(),
        ),
    ))
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

pub fn u64_sub(a: u64, b: BigUint) -> u64 {
    let b: u64 = b.try_into().unwrap();
    if a <= b {
        0
    } else {
        a - b
    }
}

pub fn u128_sub(a: u128, b: BigUint) -> u128 {
    let b: u128 = b.try_into().unwrap();
    if a <= b {
        0
    } else {
        a - b
    }
}

pub fn unwrap_only_one<T: Clone>(vec: &[T]) -> T {
    assert!(vec.len() == 1);
    vec[0].clone()
}

pub fn decode_udt_amount(data: &[u8]) -> u128 {
    u128::from_le_bytes(to_fixed_array(&data[0..16]))
}

pub fn encode_udt_amount(amount: u128) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

#[cfg(test)]
mod test {
    use super::*;

    use ckb_sdk::NetworkType;
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
    fn test_parse_address() {
        let addr = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
        let res = parse_address(addr);

        assert!(res.is_ok());
        assert_eq!(res.unwrap().network(), NetworkType::Testnet);
    }

    #[test]
    fn test_find() {
        let test = (0..10).collect::<Vec<_>>();
        test.iter()
            .for_each(|i| assert_eq!(find(i, &test), Some(*i)));
    }
}
