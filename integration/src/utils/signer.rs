use anyhow::Result;
use ckb_crypto::secp::Privkey;
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::Transaction;
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use core_rpc_types::{HashAlgorithm, SignAlgorithm, TransactionCompletionResponse};

pub fn sign_transaction(
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
                let init_witness_idx: u32 = s.signature_location.index.into();
                let init_witness = packed::WitnessArgs::from_slice(
                    witnesses[init_witness_idx as usize].raw_data().as_ref(),
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
                    let idx: u32 = idx.into();
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
    sign_transaction(transaction, pk)
}
