#![allow(clippy::mutable_key_type)]

mod error;
mod r#impl;
#[cfg(test)]
mod tests;

pub use r#impl::MercuryRpcImpl;

use ckb_jsonrpc_types::Uint64;
use ckb_types::{H160, H256};
use common::{Order, Result};
use core_rpc_types::error::MercuryRpcError;
use core_rpc_types::{
    indexer, AdjustAccountPayload, BlockInfo, DaoClaimPayload, DaoDepositPayload,
    DaoWithdrawPayload, GetAccountInfoPayload, GetAccountInfoResponse, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, MercuryInfo, PaginationResponse, QueryTransactionsPayload,
    SimpleTransferPayload, SudtIssuePayload, SyncState, TransactionCompletionResponse,
    TransferPayload, TxView,
};
use core_storage::DBInfo;
use jsonrpsee_core::RpcResult;
use jsonrpsee_proc_macros::rpc;

type InnerResult<T> = Result<T, MercuryRpcError>;

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

    #[method(name = "get_account_info")]
    async fn get_account_info(
        &self,
        payload: GetAccountInfoPayload,
    ) -> RpcResult<GetAccountInfoResponse>;

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

    #[method(name = "build_simple_transfer_transaction")]
    async fn build_simple_transfer_transaction(
        &self,
        payload: SimpleTransferPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "register_addresses")]
    async fn register_addresses(&self, addresses: Vec<String>) -> RpcResult<Vec<H160>>;

    #[method(name = "get_mercury_info")]
    async fn get_mercury_info(&self) -> RpcResult<MercuryInfo>;

    #[method(name = "get_db_info")]
    fn get_db_info(&self) -> RpcResult<DBInfo>;

    #[method(name = "build_dao_deposit_transaction")]
    async fn build_dao_deposit_transaction(
        &self,
        payload: DaoDepositPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "build_dao_withdraw_transaction")]
    async fn build_dao_withdraw_transaction(
        &self,
        payload: DaoWithdrawPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "build_dao_claim_transaction")]
    async fn build_dao_claim_transaction(
        &self,
        payload: DaoClaimPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "build_sudt_issue_transaction")]
    async fn build_sudt_issue_transaction(
        &self,
        payload: SudtIssuePayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[method(name = "get_spent_transaction")]
    async fn get_spent_transaction(&self, payload: GetSpentTransactionPayload)
        -> RpcResult<TxView>;

    #[method(name = "get_tip")]
    async fn get_tip(&self) -> RpcResult<Option<indexer::Tip>>;

    #[method(name = "get_cells")]
    async fn get_cells(
        &self,
        search_key: indexer::SearchKey,
        order: Order,
        limit: Uint64,
        after_cursor: Option<Uint64>,
    ) -> RpcResult<indexer::PaginationResponse<indexer::Cell>>;

    #[method(name = "get_cells_capacity")]
    async fn get_cells_capacity(
        &self,
        search_key: indexer::SearchKey,
    ) -> RpcResult<indexer::CellsCapacity>;

    #[method(name = "get_transactions")]
    async fn get_transactions(
        &self,
        search_key: indexer::SearchKey,
        order: Order,
        limit: Uint64,
        after_cursor: Option<Uint64>,
    ) -> RpcResult<indexer::PaginationResponse<indexer::Transaction>>;

    #[method(name = "get_ckb_uri")]
    async fn get_ckb_uri(&self) -> RpcResult<Vec<String>>;

    #[method(name = "get_live_cells_by_lock_hash")]
    async fn get_live_cells_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> RpcResult<Vec<indexer::LiveCell>>;

    #[method(name = "get_capacity_by_lock_hash")]
    async fn get_capacity_by_lock_hash(
        &self,
        lock_hash: H256,
    ) -> RpcResult<indexer::LockHashCapacity>;

    #[method(name = "get_transactions_by_lock_hash")]
    async fn get_transactions_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> RpcResult<Vec<indexer::CellTransaction>>;

    #[method(name = "get_sync_state")]
    async fn get_sync_state(&self) -> RpcResult<SyncState>;

    #[method(name = "start_profiler")]
    async fn start_profiler(&self) -> RpcResult<()>;

    #[method(name = "report_pprof")]
    async fn report_pprof(&self) -> RpcResult<()>;
}
