#![allow(clippy::mutable_key_type, clippy::upper_case_acronyms)]

pub mod ckb_client;
pub mod rpc_impl;
pub mod types;

mod error;
#[cfg(test)]
mod tests;

use types::{
    CreateWalletPayload, GetBalanceResponse, ScanBlockPayload, ScanBlockResponse,
    TransactionCompletionResponse, TransferPayload,
};

pub use ckb_client::CkbRpcClient;
pub use rpc_impl::{MercuryRpcImpl, TX_POOL_CACHE, USE_HEX_FORMAT};

use common::anyhow::Result;

use async_trait::async_trait;
use ckb_jsonrpc_types::{BlockView, LocalNode, RawTxPool, TransactionWithStatus};
use ckb_types::{core::BlockNumber, H256};
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

#[rpc(server)]
pub trait MercuryRpc {
    #[rpc(name = "get_balance")]
    fn get_balance(
        &self,
        udt_hashes: Vec<Option<H256>>,
        addr: String,
    ) -> RpcResult<Vec<GetBalanceResponse>>;

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

    #[rpc(name = "get_transaction_history")]
    fn get_transaction_history(&self, ident: String) -> RpcResult<Vec<TransactionWithStatus>>;

    #[rpc(name = "scan_deposit")]
    fn scan_deposit(&self, payload: ScanBlockPayload) -> RpcResult<ScanBlockResponse>;
}

#[async_trait]
pub trait CkbRpc {
    async fn local_node_info(&self) -> Result<LocalNode>;

    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool>;

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>>;

    async fn get_block_by_number(
        &self,
        block_number: BlockNumber,
        use_hex_format: bool,
    ) -> Result<Option<BlockView>>;
}
