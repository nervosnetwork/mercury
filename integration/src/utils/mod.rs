use anyhow::Result;
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, h256, packed, prelude::*};
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
    uri: String,
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

pub(crate) fn post_http_request(uri: String, body: &'static str) -> serde_json::Value {
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

#[test]
pub fn test_generate_rand_secp_address_pk_pair() {
    let (address, _) = generate_rand_secp_address_pk_pair();
    assert!(address.is_secp256k1())
}
