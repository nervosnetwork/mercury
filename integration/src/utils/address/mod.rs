pub mod acp;
pub mod cheque;
pub mod omni_lock;
pub mod pw_lock;

use crate::const_definition::{SIGHASH_TYPE_HASH, SUDT_DEVNET_TYPE_HASH};

use anyhow::{anyhow, Result};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*, H160, H256};
use common::{
    address::{is_acp, is_secp256k1},
    Address, AddressPayload, NetworkType,
};
use core_rpc_types::{Identity, IdentityFlag};
use crypto::digest::Digest;
use crypto::sha3::Sha3;
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
    let secp_code_hash =
        packed::Byte32::from_slice(SIGHASH_TYPE_HASH.as_bytes()).expect("impossible:");
    let payload = AddressPayload::new_full(ScriptHashType::Type, secp_code_hash, args);
    let address = Address::new(NetworkType::Testnet, payload, true);

    (address, pk)
}

pub(crate) fn new_identity_from_secp_address(address: &str) -> Result<Identity> {
    let address = Address::from_str(address).map_err(|err| anyhow!(err))?;
    if !is_secp256k1(&address) && !is_acp(&address) {
        return Err(anyhow!("not secp address"));
    }
    let script: packed::Script = address.payload().into();
    let pub_key_hash = H160::from_slice(&script.args().as_slice()[4..24])?;
    Ok(Identity::new(IdentityFlag::Ckb, pub_key_hash))
}

pub fn get_udt_hash_by_owner(owner_address: &Address) -> Result<H256> {
    let owner_script: packed::Script = owner_address.payload().into();
    let sudt_type_script = packed::ScriptBuilder::default()
        .code_hash(SUDT_DEVNET_TYPE_HASH.pack())
        .args(owner_script.calc_script_hash().raw_data().pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    Ok(sudt_type_script.calc_script_hash().unpack())
}

// for testing only
pub fn generate_rand_private_key() -> H256 {
    H256(rand::thread_rng().gen::<[u8; 32]>())
}

pub fn build_secp_address(acp_address: &Address) -> Result<Address> {
    let acp_script: packed::Script = acp_address.payload().into();
    let secp_script = packed::ScriptBuilder::default()
        .code_hash(SIGHASH_TYPE_HASH.pack())
        .args(acp_script.args())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&secp_script);
    Ok(Address::new(NetworkType::Dev, payload, true))
}

pub fn pubkey_to_eth_address(pubkey_uncompressed: &str) -> String {
    assert_eq!(130, pubkey_uncompressed.chars().count());

    let pubkey_without_prefix = pubkey_uncompressed.split_once("04").unwrap().1;
    let pubkey_without_prefix = hex::decode(pubkey_without_prefix).unwrap();
    let mut hasher = Sha3::keccak256();
    hasher.input(&pubkey_without_prefix);
    let hash = hasher.result_str();
    hash.split_at(24).1.to_string()
}
