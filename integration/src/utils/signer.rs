use crate::const_definition::{
    ANYONE_CAN_PAY_DEVNET_TYPE_HASH, CHEQUE_DEVNET_TYPE_HASH, SIGHASH_TYPE_HASH,
};

use anyhow::Result;
use ckb_crypto::secp::Privkey;
use ckb_hash::blake2b_256;
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::{Script, Transaction};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use core_rpc_types::{ScriptGroupType, TransactionCompletionResponse};
use secp256k1::{self, PublicKey, Secp256k1, SecretKey};

use std::str::FromStr;

pub fn sign_transaction(
    transaction: TransactionCompletionResponse,
    pks: &[H256],
) -> Result<Transaction> {
    let script_groups = transaction.script_groups;
    let tx_hash = transaction.tx_view.hash.0;
    let tx: packed::Transaction = transaction.tx_view.inner.into();
    let mut witnesses: Vec<packed::Bytes> = tx.witnesses().into_iter().collect();
    for script_group in script_groups {
        if script_group.group_type == ScriptGroupType::TypeScript {
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
            let init_witness = if witnesses[init_witness_idx as usize].is_empty() {
                packed::WitnessArgs::default()
            } else {
                packed::WitnessArgs::from_slice(
                    witnesses[init_witness_idx as usize].raw_data().as_ref(),
                )
                .map_err(anyhow::Error::new)?
            };

            let init_witness = init_witness
                .as_builder()
                .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                .build();
            let mut blake2b = new_blake2b();
            blake2b.update(&tx_hash);
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
            let sig = privkey.sign_recoverable(&message).expect("sign");
            witnesses[init_witness_idx as usize] = init_witness
                .as_builder()
                .lock(Some(Bytes::from(sig.serialize())).pack())
                .build()
                .as_bytes()
                .pack();
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

pub fn sign_transaction_for_cheque_of_sender(
    mut transaction: TransactionCompletionResponse,
    pk: &H256,
    cheque_input_indexes: Vec<usize>,
) -> Result<Transaction> {
    for index in cheque_input_indexes {
        let since = &mut transaction.tx_view.inner.inputs[index].since;
        *since = {
            // when sender withdraw, cheque cell since must be hardcoded as 0xA000000000000006
            11529215046068469766u64.into()
        };
    }
    let tx: packed::Transaction = transaction.tx_view.inner.into();
    let tx_view = tx.as_advanced_builder().build();
    transaction.tx_view = tx_view.into();
    sign_transaction(transaction, &[pk.to_owned()])
}

pub fn get_uncompressed_pubkey_from_pk(pk: &str) -> String {
    let secret_key = SecretKey::from_str(pk).expect("get SecretKey");
    let secp256k1: Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
    let pubkey = PublicKey::from_secret_key(&secp256k1, &secret_key);
    hex::encode(pubkey.serialize_uncompressed())
}

fn pubkey_to_secp_lock_arg(pubkey_uncompressed: &str) -> Bytes {
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

fn get_secp_lock_arg(pk: &H256) -> Bytes {
    let pubkey = get_uncompressed_pubkey_from_pk(&pk.to_string());
    pubkey_to_secp_lock_arg(&pubkey)
}

fn get_right_pk<'a>(pks: &'a [H256], script: &Script) -> Option<&'a H256> {
    if script.code_hash == CHEQUE_DEVNET_TYPE_HASH {
        return Some(&pks[0]);
    }
    let args = script.args.clone().into_bytes();
    for pk in pks {
        if get_secp_lock_arg(pk) == args {
            return Some(pk);
        }
    }
    None
}
