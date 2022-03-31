use core_rpc_types::{HashAlgorithm, SignAlgorithm, TransactionCompletionResponse};

use anyhow::Result;
use ckb_crypto::secp::Privkey;
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::Transaction;
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};

pub struct Signer {}

impl Default for Signer {
    fn default() -> Self {
        Self::new()
    }
}

impl Signer {
    fn new() -> Self {
        Signer {}
    }

    pub fn sign_transaction(
        &self,
        transaction: TransactionCompletionResponse,
        pk: &H256,
    ) -> Result<Transaction> {
        let signature_actions = transaction.signature_actions;
        let tx_hash = transaction.tx_view.hash.0;
        let tx: packed::Transaction = transaction.tx_view.inner.into();
        let mut witnesses: Vec<packed::Bytes> = tx.witnesses().into_iter().collect();

        for s in signature_actions {
            match (s.hash_algorithm, s.signature_info.algorithm) {
                (HashAlgorithm::Blake2b, SignAlgorithm::Secp256k1) => {
                    let init_witness_idx = s.signature_location.index;
                    let init_witness = packed::WitnessArgs::from_slice(
                        witnesses[init_witness_idx].raw_data().as_ref(),
                    )
                    .map_err(anyhow::Error::new)?;

                    let init_witness = init_witness
                        .as_builder()
                        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                        .build();
                    let mut blake2b = new_blake2b();
                    blake2b.update(&tx_hash);
                    blake2b.update(&(init_witness.as_bytes().len() as u64).to_le_bytes());
                    blake2b.update(&init_witness.as_bytes());
                    for idx in s.other_indexes_in_group {
                        let other_witness = &witnesses[idx];
                        blake2b.update(&(other_witness.len() as u64).to_le_bytes());
                        blake2b.update(&other_witness.as_bytes());
                    }
                    for other_witness in witnesses.iter().skip(tx.raw().inputs().len()) {
                        blake2b.update(&(other_witness.len() as u64).to_le_bytes());
                        blake2b.update(&other_witness.as_bytes());
                    }
                    let mut message = [0u8; 32];
                    blake2b.finalize(&mut message);
                    let message = H256::from(message);

                    let privkey = Privkey::from_slice(pk.as_bytes());
                    let sig = privkey.sign_recoverable(&message).expect("sign");

                    witnesses[init_witness_idx] = init_witness
                        .as_builder()
                        .lock(Some(Bytes::from(sig.serialize())).pack())
                        .build()
                        .as_bytes()
                        .pack();
                }
                (HashAlgorithm::Keccak256, SignAlgorithm::EthereumPersonal) => {
                    todo!()
                }
                _ => unreachable!(),
            }
        }
        let tx = tx
            .as_advanced_builder()
            .set_witnesses(witnesses)
            .build()
            .data();
        Ok(tx.into())
    }
}
