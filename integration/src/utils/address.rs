use crate::const_definition::{CKB_URI, MERCURY_URI, SUPER_USER_ADDRESS, SUPER_USER_PK};
use crate::mercury_types::{AssetInfo, From, JsonItem, Mode, Source, To, ToInfo, TransferPayload};
use crate::utils::instruction::generate_block;
use crate::utils::rpc_client::{CkbRpcClient, MercuryRpcClient};
use crate::utils::signer::Signer;

use anyhow::Result;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::OutputsValidator;
use ckb_types::{bytes::Bytes, core::ScriptHashType, h256, packed, prelude::*};
use common::{Address, AddressPayload, NetworkType};
use rand::Rng;

use std::str::FromStr;

pub(crate) fn generate_rand_secp_address_pk_pair() -> (Address, String) {
    // generate pubkey by privkey
    let pk = generate_rand_private_key();
    let secret_key =
        secp256k1::SecretKey::from_str(&pk).expect("impossible: fail to build secret key");
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

// for testing only
fn generate_rand_private_key() -> String {
    let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
    hex::encode(&random_bytes)
}

pub(crate) fn prepare_address_with_ckb_capacity(capacity: u64) -> Result<Address> {
    let (address, _pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(SUPER_USER_ADDRESS.to_string())],
            source: Source::Free,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: address.to_string(),
                amount: capacity.to_string(),
            }],
            mode: Mode::HoldByFrom,
        },
        pay_fee: None,
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload)?;
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &SUPER_USER_PK)?;

    // send tx to ckb node
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let tx_hash = ckb_client.send_transaction(tx, OutputsValidator::Passthrough)?;
    println!("send tx: 0x{}", tx_hash.to_string());
    for _ in 0..3 {
        generate_block()?;
    }

    Ok(address)
}

#[test]
pub fn test_generate_rand_secp_address_pk_pair() {
    let (address, _) = generate_rand_secp_address_pk_pair();
    assert!(address.is_secp256k1())
}
