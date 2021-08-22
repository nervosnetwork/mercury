#![allow(clippy::mutable_key_type, clippy::upper_case_acronyms)]

pub mod ckb_client;
pub mod rpc_impl;
pub mod types;

mod error;
#[cfg(test)]
mod tests;

use error::RpcResult;
use types::{
    BlockInfo, GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload,
    TransactionCompletionResponse, TransferPayload,
};

pub use ckb_client::CkbRpcClient;
pub use rpc_impl::{CURRENT_BLOCK_NUMBER, TX_POOL_CACHE, USE_HEX_FORMAT};

use common::{anyhow::Result, PaginationResponse};

use async_trait::async_trait;
use ckb_jsonrpc_types::{BlockView, LocalNode, RawTxPool, TransactionWithStatus};
use ckb_types::{core::BlockNumber, H160, H256};
use jsonrpsee_proc_macros::rpc;

use crate::types::{
    AdjustAccountPayload, AdvanceQueryPayload, DepositPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, MercuryInfo, QueryResponse, QueryTransactionsPayload,
    SmartTransferPayload, TxView, WithdrawPayload,
};

#[rpc(server)]
pub trait MercuryRpc {
    #[method(name = "get_balance")]
    async fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse>;

    #[method(name = "get_block_info")]
    async fn get_block_info(&self, payload: GetBlockInfoPayload) -> RpcResult<BlockInfo>;

    #[method(name = "get_transaction_info")]
    async fn get_transaction_info(&self, tx_hash: H256) -> RpcResult<GetTransactionInfoResponse>;

    #[method(name = "query_transactions")]
    async fn query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> RpcResult<PaginationResponse<TxView>>;

    #[method(name = "build_adjust_account_transaction")]
    async fn build_adjust_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> RpcResult<Option<TransactionCompletionResponse>>;

    #[method(name = "build_transfer_transaction")]
    async fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "build_smart_transfer_transaction")]
    async fn build_smart_transfer_transaction(
        &self,
        payload: SmartTransferPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "register_address")]
    async fn register_addresses(&self, addresses: Vec<String>) -> RpcResult<Vec<H160>>;

    #[method(name = "get_mercury_info")]
    fn get_mercury_info(&self) -> RpcResult<MercuryInfo>;

    #[method(name = "get_db_info")]
    fn get_db_info(&self) -> RpcResult<DBInfo>;

    #[method(name = "build_deposit_transaction")]
    async fn build_deposit_transaction(
        &self,
        payload: DepositPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "build_withdraw_transaction")]
    async fn build_withdraw_transaction(
        &self,
        payload: WithdrawPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "get_spent_transaction")]
    async fn get_spent_transaction(&self, payload: GetSpentTransactionPayload)
        -> RpcResult<TxView>;

    #[method(name = "advance_query")]
    async fn advance_query(
        &self,
        payload: AdvanceQueryPayload,
    ) -> RpcResult<PaginationResponse<QueryResponse>>;
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

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>>;
}
