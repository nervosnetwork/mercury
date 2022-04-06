use crate::const_definition::SUDT_DEVNET_TYPE_HASH;

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

fn _caculate_type_hash(code_hash: &str, args: &str, script_hash_type: ScriptHashType) -> H256 {
    let code_hash = H256::from_str(code_hash).unwrap();
    let args = H256::from_str(args).unwrap();
    let script = packed::Script::new_builder()
        .hash_type(script_hash_type.into())
        .code_hash(code_hash.pack())
        .args(ckb_types::bytes::Bytes::from(args.as_bytes().to_owned()).pack())
        .build();
    script.calc_script_hash().unpack()
}

#[test]
fn test_caculate_lock_hash() {
    let code_hash = "00000000000000000000000000000000000000000000000000545950455f4944";

    // sudt
    let args = "314f67c0ffd0c6fbffe886f03c6b00b42e4e66e3e71d32a66b8a38d69e6a4250";
    let script_hash_type = ScriptHashType::Type;
    let script_hash = _caculate_type_hash(code_hash, args, script_hash_type);
    assert_eq!(
        "9c6933d977360f115a3e9cd5a2e0e475853681b80d775d93ad0f8969da343e56",
        &script_hash.to_string()
    );

    // anyone_can_pay
    let args = "57fdfd0617dcb74d1287bb78a7368a3a4bf9a790cfdcf5c1a105fd7cb406de0d";
    let script_hash_type = ScriptHashType::Type;
    let script_hash = _caculate_type_hash(code_hash, args, script_hash_type);
    assert_eq!(
        "6283a479a3cf5d4276cd93594de9f1827ab9b55c7b05b3d28e4c2e0a696cfefd",
        &script_hash.to_string()
    );
}

#[test]
fn test_generate_rand_secp_address_pk_pair() {
    let (address, _) = generate_rand_secp_address_pk_pair();
    assert!(address.is_secp256k1())
}
