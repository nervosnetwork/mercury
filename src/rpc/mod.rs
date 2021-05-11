pub mod rpc_impl;
mod types;

#[cfg(test)]
mod tests;

use types::SMTUpdateItem;

use ckb_jsonrpc_types::{Transaction, TransactionView};
use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

pub use rpc_impl::{MercuryRpcImpl, TX_POOL_CACHE};
use types::{
    CreateWalletPayload, GetBalanceResponse, TransactionCompletionResponse, TransferPayload,
};

#[rpc(server)]
pub trait MercuryRpc {
    #[rpc(name = "get_balance")]
    fn get_balance(&self, udt_hash: Option<H256>, addr: String) -> RpcResult<GetBalanceResponse>;

    #[rpc(name = "is_in_rce_list")]
    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool>;

    #[rpc(name = "build_transfer_transaction")]
    fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[rpc(name = "build_wallet_creation_transaction")]
    fn build_wallet_creation_transaction(
        &self,
        payload: CreateWalletPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[rpc(name = "transfer_with_rce_completion")]
    fn transfer_with_rce_completion(&self, transaction: Transaction) -> RpcResult<TransactionView>;

    #[rpc(name = "rce_update_completion")]
    fn rce_update_completion(
        &self,
        transaction: Transaction,
        smt_update: Vec<SMTUpdateItem>,
    ) -> RpcResult<TransactionView>;
}
