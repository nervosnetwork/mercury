use crate::utils::mercury_types::TransactionCompletionResponse;

use anyhow::Result;
use ckb_jsonrpc_types::TransactionView;

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
        tx: TransactionCompletionResponse,
        _pk: &str,
    ) -> Result<TransactionView> {
        Ok(tx.tx_view)
    }
}
