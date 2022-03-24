pub mod client;
pub mod const_definition;
pub mod mercury_types;
pub mod signer;

use crate::utils::client::{generate_block, handle_response, RpcClient};
use crate::utils::const_definition::{CKB_URI, MERCURY_URI, SUPER_USER_ADDRESS, SUPER_USER_PK};
use crate::utils::mercury_types::{
    AssetInfo, From, JsonItem, Mode, Source, To, ToInfo, TransactionCompletionResponse,
    TransferPayload,
};
use crate::utils::signer::Signer;

use anyhow::Result;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::OutputsValidator;
use ckb_types::{bytes::Bytes, core::ScriptHashType, h256, packed, prelude::*, H256};
use common::{Address, AddressPayload, NetworkType};
use core::panic;
use rand::Rng;
use serde_json;

use std::ffi::OsStr;
use std::process::{Child, Command};
use std::str::FromStr;

pub(crate) fn run<I, S>(bin: &str, args: I) -> Result<Child>
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<OsStr>,
{
    let child = Command::new(bin.to_owned())
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .spawn()
        .expect("run command");
    Ok(child)
}

pub(crate) fn try_post_http_request(
    uri: &'static str,
    body: &'static str,
) -> Result<reqwest::blocking::Response> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(uri)
        .header("content-type", "application/json")
        .body(body)
        .send();
    resp.map_err(anyhow::Error::new)
}

pub(crate) fn post_http_request(uri: &'static str, body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(uri)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .unwrap();
    if !resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();

    serde_json::from_str(&text).unwrap()
}

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
    let mercury_client = RpcClient::new(MERCURY_URI.to_string());
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
    let request =
        mercury_client.build_request("build_transfer_transaction".to_string(), vec![payload])?;
    let response = mercury_client.rpc_exec(&request)?;
    let tx: TransactionCompletionResponse = handle_response(response)?;
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &SUPER_USER_PK)?;

    // send tx to ckb node
    let ckb_client = RpcClient::new(CKB_URI.to_string());
    let request = ckb_client.build_request(
        "send_transaction".to_string(),
        (tx, OutputsValidator::Passthrough),
    )?;
    let response = ckb_client.rpc_exec(&request)?;
    println!("{:?}", response);
    let _tx_hash: H256 = handle_response(response)?;
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
