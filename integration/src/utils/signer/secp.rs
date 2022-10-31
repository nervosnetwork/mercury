use super::get_uncompressed_pubkey_from_pk;

use ckb_crypto::secp::{Privkey, Signature};
use ckb_hash::blake2b_256;
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::TransactionView;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use core_rpc_types::ScriptGroup;

use std::convert::From;
use std::str::FromStr;

pub fn sign_secp(
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
    let mut blake2b = new_blake2b();
    blake2b.update(tx_hash);
    blake2b.update(&(init_witness.as_bytes().len() as u64).to_le_bytes());
    blake2b.update(&init_witness.as_bytes());
    for idx in &script_group.input_indices[1..] {
        let idx: u32 = (*idx).into();
        let other_witness = witnesses[idx as usize].raw_data();
        blake2b.update(&(other_witness.len() as u64).to_le_bytes());
        blake2b.update(&other_witness);
    }
    for other_witness in witnesses.iter().skip(tx.raw().inputs().len()) {
        let other_witness = other_witness.raw_data();
        blake2b.update(&(other_witness.len() as u64).to_le_bytes());
        blake2b.update(&other_witness);
    }
    let mut message = [0u8; 32];
    blake2b.finalize(&mut message);
    let message = H256::from(message);

    let privkey = Privkey::from_slice(pk.as_bytes());
    privkey.sign_recoverable(&message).expect("sign")
}

pub fn pubkey_to_secp_lock_arg(pubkey_uncompressed: &str) -> Bytes {
    let pubkey = secp256k1::PublicKey::from_str(pubkey_uncompressed).unwrap();
    let pubkey_compressed = &pubkey.serialize()[..];

    assert_eq!(33, pubkey_compressed.len());

    let pubkey_hash = blake2b_256(pubkey_compressed);

    assert_eq!(32, pubkey_hash.len());

    let pubkey_hash = &pubkey_hash[0..20];
    let pubkey_hash =
        H160::from_slice(pubkey_hash).expect("Generate hash(H160) from pubkey failed");
    Bytes::from(pubkey_hash.as_bytes().to_vec())
}

pub fn get_secp_lock_arg(pk: &H256) -> Bytes {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    pubkey_to_secp_lock_arg(&pubkey)
}
