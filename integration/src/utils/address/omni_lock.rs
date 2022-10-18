use crate::const_definition::OMNI_LOCK_DEVNET_TYPE_HASH;
use crate::utils::address::*;
use crate::utils::instruction::prepare_ckb_capacity;
use crate::utils::signer::get_uncompressed_pubkey_from_pk;

use common::{Address, AddressPayload, NetworkType};

use anyhow::Result;
use ckb_jsonrpc_types::OutPoint;
use ckb_types::H256;
use ckb_types::{core::ScriptHashType, packed, prelude::*};

use std::str::FromStr;

pub(crate) fn generate_omni_ethereum_address_pk_pair() -> (Address, H256) {
    let pk = generate_rand_private_key();
    let address = build_omni_ethereum_address(&pk);
    (address, pk)
}

pub fn build_omni_ethereum_address(pk: &H256) -> Address {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_eth_address(&pubkey);
    let args = H160::from_str(&args).expect("parse args");
    let identity = Identity::new(IdentityFlag::Ethereum, args);
    let script = packed::ScriptBuilder::default()
        .code_hash(OMNI_LOCK_DEVNET_TYPE_HASH.pack())
        .args(identity.0.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&script);
    Address::new(NetworkType::Dev, payload, true)
}

pub(crate) fn prepare_omni_lock_address_with_capacity(
    capacity: u64,
) -> Result<(Address, H256, OutPoint)> {
    let (address, pk) = generate_omni_ethereum_address_pk_pair();
    let out_point = prepare_ckb_capacity(&address, capacity)?;
    Ok((address, pk, out_point))
}
