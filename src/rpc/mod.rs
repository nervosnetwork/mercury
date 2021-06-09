pub mod rpc_impl;
mod types;

#[cfg(test)]
mod tests;

use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

pub use rpc_impl::MercuryRpcImpl;
use types::{CreateWalletPayload, GetBalanceResponse, TransferCompletionResponse, TransferPayload};

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
    ) -> RpcResult<TransferCompletionResponse>;

    #[rpc(name = "build_wallet_creation_transaction")]
    fn build_wallet_creation_transaction(
        &self,
        payload: CreateWalletPayload,
    ) -> RpcResult<TransferCompletionResponse>;
}
