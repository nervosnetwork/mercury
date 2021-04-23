use crate::error::MercuryError;

use anyhow::Result;
use ckb_sdk::{Address, AddressPayload, AddressType, CodeHashIndex};
use ckb_types::H160;

use std::str::FromStr;

pub fn parse_address(input: &str) -> Result<Address> {
    Address::from_str(input).map_err(|e| MercuryError::ParseCKBAddressError(e).into())
}

pub fn to_short_address(input: &Address) -> Result<Address> {
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
}
