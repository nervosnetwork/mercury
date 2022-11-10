use crate::const_definition::OMNI_LOCK_DEVNET_TYPE_HASH;
use crate::utils::address::*;
use crate::utils::instruction::prepare_ckb_capacity;
use crate::utils::signer::get_uncompressed_pubkey_from_pk;

use common::{Address, AddressPayload, NetworkType};

use anyhow::Result;
use ckb_jsonrpc_types::OutPoint;
use ckb_types::H256;
use ckb_types::{core::ScriptHashType, packed, prelude::*};
use extension_lock::omni_lock::{get_acp_script, script_to_identity, OmniLockFlags};
use std::str::FromStr;

pub(crate) fn prepare_omni_ethereum_address_with_capacity(
    capacity: u64,
) -> Result<(Identity, Address, H256, OutPoint)> {
    let (identity, address, pk) = generate_omni_ethereum_address_pk_pair();
    let out_point = prepare_ckb_capacity(&address, capacity)?;
    Ok((identity, address, pk, out_point))
}

pub(crate) fn prepare_omni_secp_address_with_capacity(
    capacity: u64,
) -> Result<(Identity, Address, H256, OutPoint)> {
    let (identity, address, pk) = generate_omni_secp_address_pk_pair();
    let out_point = prepare_ckb_capacity(&address, capacity)?;
    Ok((identity, address, pk, out_point))
}

pub fn generate_omni_secp_address_pk_pair() -> (Identity, Address, H256) {
    let pk = generate_rand_private_key();
    let (identity, address) = build_omni_secp_address(&pk);
    (identity, address, pk)
}

pub(crate) fn generate_omni_ethereum_address_pk_pair() -> (Identity, Address, H256) {
    let pk = generate_rand_private_key();
    let (identity, address) = build_omni_ethereum_address(&pk);
    (identity, address, pk)
}

pub fn build_omni_secp_address(pk: &H256) -> (Identity, Address) {
    let args = generate_secp_args_from_pk(pk).unwrap();
    let identity = Identity::new(IdentityFlag::Ckb, args);
    let mut args = vec![];
    args.extend(identity.0);
    args.extend(vec![OmniLockFlags::OFF.bits()]); // omni lock args
    let script = packed::ScriptBuilder::default()
        .code_hash(OMNI_LOCK_DEVNET_TYPE_HASH.pack())
        .args(args.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&script);
    (identity, Address::new(NetworkType::Dev, payload, true))
}

pub fn build_omni_ethereum_address(pk: &H256) -> (Identity, Address) {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_eth_address(&pubkey);
    let args = H160::from_str(&args).expect("parse args");
    let identity = Identity::new(IdentityFlag::Ethereum, args);
    let mut args = vec![];
    args.extend(identity.0);
    args.extend(vec![0u8]); // omni lock args
    let script = packed::ScriptBuilder::default()
        .code_hash(OMNI_LOCK_DEVNET_TYPE_HASH.pack())
        .args(args.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&script);
    (identity, Address::new(NetworkType::Dev, payload, true))
}

pub fn build_omni_acp_account_address(omni_address: &Address) -> Result<Address> {
    let omni_script: packed::Script = omni_address.payload().into();
    let script = get_acp_script(omni_script).ok_or_else(|| anyhow!("get_acp_script"))?;
    let payload = AddressPayload::from_script(&script);
    Ok(Address::new(NetworkType::Dev, payload, true))
}

pub fn new_identity_from_omni_address(address: &str) -> Result<Identity> {
    let address = Address::from_str(address).map_err(|err| anyhow!(err))?;
    let script: packed::Script = address.payload().into();
    let identity = script_to_identity(&script);
    identity.ok_or_else(|| anyhow!("get identiry"))
}
