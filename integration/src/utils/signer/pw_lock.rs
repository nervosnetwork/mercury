use super::{get_uncompressed_pubkey_from_pk, pubkey_to_eth_address};

use ckb_crypto::secp::{Privkey, Signature};
use ckb_jsonrpc_types::TransactionView;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use core_rpc_types::ScriptGroup;
use crypto::digest::Digest;
use crypto::sha3::Sha3;

use std::convert::From;
use std::str::FromStr;

pub fn sign_ethereum(
    zero_lock: Bytes,
    tx_view: &TransactionView,
    script_group: &ScriptGroup,
    pk: &H256,
) -> Signature {
    let tx_hash = &tx_view.hash.0;
    let tx: packed::Transaction = tx_view.inner.clone().into();
    let witnesses: Vec<packed::Bytes> = tx.witnesses().into_iter().collect();
    let init_witness_idx: u32 = script_group.input_indices[0].into();

    let init_witness = if witnesses[init_witness_idx as usize].is_empty() {
        packed::WitnessArgs::default()
    } else {
        packed::WitnessArgs::from_slice(witnesses[init_witness_idx as usize].raw_data().as_ref())
            .map_err(anyhow::Error::new)
            .expect("get init_witness")
    };

    let init_witness = init_witness
        .as_builder()
        .lock(Some(zero_lock).pack())
        .build();
    let mut hasher = Sha3::keccak256();
    hasher.input(tx_hash);
    hasher.input(&(init_witness.as_bytes().len() as u64).to_le_bytes());
    hasher.input(&init_witness.as_bytes());
    for idx in &script_group.input_indices[1..] {
        let idx: u32 = (*idx).into();
        let other_witness = witnesses[idx as usize].raw_data();
        hasher.input(&(other_witness.len() as u64).to_le_bytes());
        hasher.input(&other_witness);
    }
    for other_witness in witnesses.iter().skip(tx.raw().inputs().len()) {
        let other_witness = other_witness.raw_data();
        hasher.input(&(other_witness.len() as u64).to_le_bytes());
        hasher.input(&other_witness);
    }
    let mut message = [0u8; 32];
    hasher.result(&mut message);

    hash_personal_message(&mut message);

    let message = H256::from(message);

    let privkey = Privkey::from_slice(pk.as_bytes());
    privkey.sign_recoverable(&message).expect("sign")
}

pub fn hash_personal_message(message: &mut [u8; 32]) {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len())
        .as_bytes()
        .to_vec();
    let new = [prefix, message.to_vec()].concat();

    let mut hasher = Sha3::keccak256();
    hasher.input(&new);
    hasher.result(message);
}

pub fn get_pw_lock_arg(pk: &H256) -> Bytes {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_eth_address(&pubkey);
    let args = H160::from_str(&args).expect("get args");
    Bytes::copy_from_slice(args.as_bytes())
}
