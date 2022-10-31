use super::sign_transaction;

use anyhow::Result;
use ckb_jsonrpc_types::Transaction;
use ckb_types::{packed, H256};
use core_rpc_types::TransactionCompletionResponse;

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
