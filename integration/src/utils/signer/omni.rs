use super::pw_lock::hash_personal_message;
use super::secp::pubkey_to_secp_lock_arg;
use super::{get_uncompressed_pubkey_from_pk, pubkey_to_eth_address};

use anyhow::Result;
use ckb_crypto::secp::{Privkey, Signature};
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::TransactionView;
use ckb_types::{
    bytes::Bytes,
    packed::{self, BytesOpt},
    prelude::*,
    H160, H256,
};
use core_rpc_types::{Identity, ScriptGroup};
use extension_lock::omni_lock::OmniLockWitnessLock;

use std::str::FromStr;

pub fn omni_script_to_identity(script: &packed::Script) -> Option<Identity> {
    let flag = script.args().as_slice()[4..5].to_vec()[0].try_into().ok()?;
    let hash = H160::from_slice(&script.args().as_slice()[5..25]).ok()?;
    Some(Identity::new(flag, hash))
}

/// Build proper witness lock
pub fn _build_witness_lock(orig_lock: BytesOpt, signature: Bytes) -> Result<Bytes> {
    let lock_field = orig_lock.to_opt().map(|data| data.raw_data());
    let omnilock_witnesslock = if let Some(lock_field) = lock_field {
        OmniLockWitnessLock::from_slice(lock_field.as_ref())?
    } else {
        OmniLockWitnessLock::default()
    };

    Ok(omnilock_witnesslock
        .as_builder()
        .signature(Some(signature).pack())
        .build()
        .as_bytes())
}

pub fn get_omni_secp_arg(pk: &H256) -> Bytes {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_secp_lock_arg(&pubkey).to_vec();
    let mut auth = vec![0u8];
    auth.extend_from_slice(&args);
    Bytes::copy_from_slice(&auth)
}

pub fn get_omni_eth_arg(pk: &H256) -> Bytes {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    let args = pubkey_to_eth_address(&pubkey);
    let args = H160::from_str(&args).expect("get args");
    let mut auth = vec![1u8];
    auth.extend_from_slice(args.as_bytes());
    Bytes::copy_from_slice(&auth)
}

pub fn sign_omni_ethereum(
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

    hash_personal_message(&mut message);

    let message = H256::from(message);

    let privkey = Privkey::from_slice(pk.as_bytes());
    privkey.sign_recoverable(&message).expect("sign")
}
