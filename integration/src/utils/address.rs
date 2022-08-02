use crate::const_definition::{
    ANYONE_CAN_PAY_DEVNET_TYPE_HASH, CHEQUE_DEVNET_TYPE_HASH, PW_LOCK_DEVNET_TYPE_HASH,
    SIGHASH_TYPE_HASH, SUDT_DEVNET_TYPE_HASH,
};
use crate::utils::signer::get_uncompressed_pubkey_from_pk;

use anyhow::{anyhow, Result};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*, H160, H256};
use common::{
    address::{is_acp, is_secp256k1},
    hash::blake2b_160,
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
fn generate_rand_private_key() -> H256 {
    H256(rand::thread_rng().gen::<[u8; 32]>())
}

pub fn build_cheque_address(
    receiver_address: &Address,
    sender_address: &Address,
) -> Result<Address> {
    if !is_secp256k1(receiver_address) || !is_secp256k1(sender_address) {
        return Err(anyhow!("can't get cheque address"));
    }
    let receiver_script: packed::Script = receiver_address.payload().into();
    let sender_script: packed::Script = sender_address.payload().into();
    let mut args = blake2b_160(receiver_script.as_slice()).to_vec();
    let sender = blake2b_160(sender_script.as_slice());
    args.extend_from_slice(&sender);
    let sudt_type_script = packed::ScriptBuilder::default()
        .code_hash(CHEQUE_DEVNET_TYPE_HASH.pack())
        .args(args.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&sudt_type_script);
    Ok(Address::new(NetworkType::Dev, payload, true))
}

pub fn build_acp_address(secp_address: &Address) -> Result<Address> {
    let secp_script: packed::Script = secp_address.payload().into();
    let anyone_can_pay_script = packed::ScriptBuilder::default()
        .code_hash(ANYONE_CAN_PAY_DEVNET_TYPE_HASH.pack())
        .args(secp_script.args())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&anyone_can_pay_script);
    Ok(Address::new(NetworkType::Dev, payload, true))
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

pub fn build_pw_lock_address(pk: &H256) -> Address {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_eth_address(&pubkey);
    let args = H160::from_str(&args).expect("parse args");
    let script = packed::ScriptBuilder::default()
        .code_hash(PW_LOCK_DEVNET_TYPE_HASH.pack())
        .args(args.0.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&script);
    Address::new(NetworkType::Dev, payload, true)
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

pub(crate) fn generate_rand_pw_address_pk_pair() -> (Address, H256) {
    let pk = generate_rand_private_key();
    let address = build_pw_lock_address(&pk);
    (address, pk)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build_addresses() {
        let _ = common::lazy::SECP256K1_CODE_HASH.set(SIGHASH_TYPE_HASH);

        let (address, _) = generate_rand_secp_address_pk_pair();
        assert!(is_secp256k1(&address));

        let sender = Address::from_str("ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9").unwrap();
        let receiver = Address::from_str("ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld").unwrap();
        let cheque = build_cheque_address(&receiver, &sender).unwrap();
        assert_eq!("ckt1qqdpunl0xn6es2gx7azmqj870vggjer7sg6xqa8q7vkzan3xea43uqt6g2dxvxxjtdhfvfs0f67gwzgrcrfg3gj9yywse6zu05ez3s64xmtdkl6074rac6q3f7cvk".to_string(), cheque.to_string());

        let address_secp =
            Address::from_str("ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld").unwrap();
        let acp_address = build_acp_address(&address_secp).unwrap();
        assert_eq!("ckt1qp3g8fre50846snkekf4jn0f7xp84wd4t3astv7j3exzuznfdnl06qv6e65dqy3kfslr3j2cdh4enhyqeqyawysf7sf4c".to_string(), acp_address.to_string());
    }
}
