use super::secp::pubkey_to_secp_lock_arg;
use super::{get_uncompressed_pubkey_from_pk, pubkey_to_eth_address};

use anyhow::Result;
use ckb_types::{
    bytes::Bytes,
    packed::{self, BytesOpt},
    prelude::*,
    H160, H256,
};
use core_rpc_types::Identity;
use core_storage::OmniLockWitnessLock;

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
