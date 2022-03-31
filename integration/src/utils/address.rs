use anyhow::{anyhow, Result};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, h256, packed, prelude::*, H160, H256};
use common::{Address, AddressPayload, NetworkType};
use core_rpc_types::{Identity, IdentityFlag};
use rand::Rng;

use std::str::FromStr;

pub(crate) fn generate_rand_secp_address_pk_pair() -> (Address, H256) {
    // generate pubkey by privkey
    let pk = generate_rand_private_key();
    let secret_key = secp256k1::SecretKey::from_str(&pk.to_string())
        .expect("impossible: fail to build secret key");
    let secp256k1: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
    let pubkey = secp256k1::PublicKey::from_secret_key(&secp256k1, &secret_key);

    // pubkey hash
    let pubkey = &pubkey.serialize()[..];
    let pubkey_hash = blake2b_256(pubkey);

    // generate args by pubkey hash
    let args = Bytes::from(pubkey_hash[0..20].to_vec());

    // secp address
    let secp_code_hash = packed::Byte32::from_slice(
        h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").as_bytes(),
    )
    .expect("impossible:");
    let payload = AddressPayload::new_full(ScriptHashType::Type, secp_code_hash, args);
    let address = Address::new(NetworkType::Testnet, payload, true);

    (address, pk)
}

pub(crate) fn new_identity_from_secp_address(address: &str) -> Result<Identity> {
    let address = Address::from_str(address).expect("parse address");
    if !address.is_secp256k1() {
        return Err(anyhow!("not secp address"));
    }
    let script: packed::Script = address.payload().into();
    let pub_key_hash = H160::from_slice(&script.args().as_slice()[4..24])?;
    Ok(Identity::new(IdentityFlag::Ckb, pub_key_hash))
}

// for testing only
fn generate_rand_private_key() -> H256 {
    H256(rand::thread_rng().gen::<[u8; 32]>())
}

#[test]
fn test_generate_rand_secp_address_pk_pair() {
    let (address, _) = generate_rand_secp_address_pk_pair();
    assert!(address.is_secp256k1())
}
