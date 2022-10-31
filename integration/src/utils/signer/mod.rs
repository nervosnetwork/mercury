mod cheque;
mod omni;
mod pw_lock;
mod secp;

pub use cheque::sign_transaction_for_cheque_of_sender;
use pw_lock::get_pw_lock_arg;
use secp::get_secp_lock_arg;

use omni::{get_omni_eth_arg, get_omni_secp_arg, omni_script_to_identity};
use pw_lock::sign_pw_lock;
use secp::sign_secp;

use crate::const_definition::{
    ANYONE_CAN_PAY_DEVNET_TYPE_HASH, CHEQUE_DEVNET_TYPE_HASH, OMNI_LOCK_DEVNET_TYPE_HASH,
    PW_LOCK_DEVNET_TYPE_HASH, SIGHASH_TYPE_HASH,
};
use crate::utils::address::pubkey_to_eth_address;

use anyhow::Result;
use ckb_jsonrpc_types::{Script, Transaction};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use core_rpc_types::{IdentityFlag, ScriptGroupType, TransactionCompletionResponse};
use core_storage::OmniLockWitnessLock;
use secp256k1::{self, PublicKey, Secp256k1, SecretKey};

use std::convert::From;
use std::str::FromStr;

pub fn sign_transaction(
    transaction: TransactionCompletionResponse,
    pks: &[H256],
) -> Result<Transaction> {
    let script_groups = transaction.script_groups;
    let tx: packed::Transaction = transaction.tx_view.inner.clone().into();
    let mut witnesses: Vec<packed::Bytes> = tx.witnesses().into_iter().collect();
    for script_group in script_groups {
        if script_group.group_type == ScriptGroupType::Type {
            continue;
        }
        let pk = if let Some(pk) = get_right_pk(pks, &script_group.script) {
            pk
        } else {
            continue;
        };
        let init_witness_idx: u32 = script_group.input_indices[0].into();
        if witnesses[init_witness_idx as usize].to_string() == packed::Bytes::default().to_string()
        {
            continue;
        }
        if script_group.script.code_hash == SIGHASH_TYPE_HASH
            || script_group.script.code_hash == ANYONE_CAN_PAY_DEVNET_TYPE_HASH
            || script_group.script.code_hash == CHEQUE_DEVNET_TYPE_HASH
        {
            let zero_lock = Bytes::from(vec![0u8; 65]);

            let sig = sign_secp(zero_lock, &transaction.tx_view, &script_group, pk);

            // Put signature into witness
            let current_witness = if witnesses[init_witness_idx as usize].is_empty() {
                packed::WitnessArgs::default()
            } else {
                packed::WitnessArgs::from_slice(
                    witnesses[init_witness_idx as usize].raw_data().as_ref(),
                )
                .map_err(anyhow::Error::new)?
            };

            let witness_lock = Some(Bytes::from(sig.serialize())).pack();

            witnesses[init_witness_idx as usize] = current_witness
                .as_builder()
                .lock(witness_lock)
                .build()
                .as_bytes()
                .pack();
        } else if script_group.script.code_hash == PW_LOCK_DEVNET_TYPE_HASH {
            let zero_lock = Bytes::from(vec![0u8; 65]);

            let sig = sign_pw_lock(zero_lock, &transaction.tx_view, &script_group, pk);

            // Put signature into witness
            let current_witness = if witnesses[init_witness_idx as usize].is_empty() {
                packed::WitnessArgs::default()
            } else {
                packed::WitnessArgs::from_slice(
                    witnesses[init_witness_idx as usize].raw_data().as_ref(),
                )
                .map_err(anyhow::Error::new)?
            };

            let witness_lock = Some(Bytes::from(sig.serialize())).pack();

            witnesses[init_witness_idx as usize] = current_witness
                .as_builder()
                .lock(witness_lock)
                .build()
                .as_bytes()
                .pack();
        } else if script_group.script.code_hash == OMNI_LOCK_DEVNET_TYPE_HASH {
            let ident = omni_script_to_identity(&script_group.script.clone().into()).unwrap();
            let (flag, _pubkey_hash) = ident.parse().unwrap();
            match flag {
                IdentityFlag::Ckb => {
                    let witness_lock = OmniLockWitnessLock::new_builder()
                        .signature(Some(Bytes::from(vec![0u8; 65])).pack())
                        .build();
                    let len = witness_lock.as_bytes().len();
                    let zero_lock = Bytes::from(vec![0u8; len]);

                    let sig = sign_secp(zero_lock, &transaction.tx_view, &script_group, pk);

                    // Put signature into witness
                    let current_witness = if witnesses[init_witness_idx as usize].is_empty() {
                        packed::WitnessArgs::default()
                    } else {
                        packed::WitnessArgs::from_slice(
                            witnesses[init_witness_idx as usize].raw_data().as_ref(),
                        )
                        .map_err(anyhow::Error::new)?
                    };

                    let orig_lock = current_witness.lock();
                    let lock_field = orig_lock.to_opt().map(|data| data.raw_data());
                    let omnilock_witnesslock = if let Some(lock_field) = lock_field {
                        OmniLockWitnessLock::from_slice(lock_field.as_ref())?
                    } else {
                        OmniLockWitnessLock::default()
                    };
                    let omnilock_witnesslock = omnilock_witnesslock
                        .as_builder()
                        .signature(Some(Bytes::from(sig.serialize())).pack())
                        .build();
                    let witness_lock = Some(omnilock_witnesslock.as_bytes()).pack();

                    witnesses[init_witness_idx as usize] = current_witness
                        .as_builder()
                        .lock(witness_lock)
                        .build()
                        .as_bytes()
                        .pack();
                }
                IdentityFlag::Ethereum => {}
                _ => {}
            }
        } else {
            todo!()
        }
    }

    let tx = tx
        .as_advanced_builder()
        .set_witnesses(witnesses)
        .build()
        .data();
    Ok(tx.into())
}

pub fn get_uncompressed_pubkey_from_pk(pk: &str) -> String {
    let secret_key = SecretKey::from_str(pk).expect("get SecretKey");
    let secp256k1: Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
    let pubkey = PublicKey::from_secret_key(&secp256k1, &secret_key);
    hex::encode(pubkey.serialize_uncompressed())
}

fn get_right_pk<'a>(pks: &'a [H256], script: &Script) -> Option<&'a H256> {
    if script.code_hash == CHEQUE_DEVNET_TYPE_HASH {
        return Some(&pks[0]);
    }
    let args = script.args.clone().into_bytes();
    for pk in pks {
        if get_secp_lock_arg(pk) == args || get_pw_lock_arg(pk) == args {
            return Some(pk);
        }
        if args.len() > 21
            && (get_omni_secp_arg(pk).to_vec() == args.to_vec()[0..21]
                || get_omni_eth_arg(pk).to_vec() == args.to_vec()[0..21])
        {
            return Some(pk);
        }
    }
    None
}
