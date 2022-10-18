use crate::const_definition::SIGHASH_TYPE_HASH;
use crate::utils::address::generate_rand_private_key;

use common::{Address, AddressPayload, NetworkType};

use anyhow::Result;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::OutPoint;
use ckb_types::H256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*};

use std::str::FromStr;

pub(crate) fn _prepare_omni_lock_address_with_capacity(
    _capacity: u64,
) -> Result<(Address, H256, OutPoint)> {
    todo!()
}

pub(crate) fn _generate_omni_lock_address_pk_pair(
    _auth: Bytes,
    _omni_args: Bytes,
) -> (Address, H256) {
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
    let secp_code_hash =
        packed::Byte32::from_slice(SIGHASH_TYPE_HASH.as_bytes()).expect("impossible:");
    let payload = AddressPayload::new_full(ScriptHashType::Type, secp_code_hash, args);
    let address = Address::new(NetworkType::Testnet, payload, true);

    (address, pk)
}
