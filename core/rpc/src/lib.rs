#![allow(clippy::mutable_key_type, clippy::upper_case_acronyms)]

pub mod ckb_client;
pub mod rpc_impl;
pub mod types;

mod error;
#[cfg(test)]
mod tests;

use error::{RpcErrorMessage, RpcResult};
use types::{
    indexer, indexer_legacy, AdjustAccountPayload, BlockInfo, DaoClaimPayload, DaoDepositPayload,
    DaoWithdrawPayload, GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload,
    GetSpentTransactionPayload, GetTransactionInfoResponse, MercuryInfo, QueryTransactionsPayload,
    SmartTransferPayload, TransactionCompletionResponse, TransferPayload, TxView,
};

pub use ckb_client::CkbRpcClient;
pub use rpc_impl::{MercuryRpcImpl, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER, TX_POOL_CACHE};

use common::{PaginationResponse, Result};
use core_storage::DBInfo;
use core_synchronization::SyncAdapter;

use async_trait::async_trait;
use ckb_jsonrpc_types::{
    BlockView, EpochView, LocalNode, RawTxPool, TransactionWithStatus, Uint64,
};
use ckb_types::{bytes::Bytes, core, core::BlockNumber, H160, H256};
use jsonrpsee_proc_macros::rpc;

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

    #[method(name = "get_spent_transaction")]
    async fn get_spent_transaction(&self, payload: GetSpentTransactionPayload)
        -> RpcResult<TxView>;

    #[method(name = "get_tip")]
    async fn get_tip(&self) -> RpcResult<Option<indexer::Tip>>;

    #[method(name = "get_cells")]
    async fn get_cells(
        &self,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
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
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
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
    ) -> RpcResult<Vec<indexer_legacy::LiveCell>>;

    #[method(name = "get_capacity_by_lock_hash")]
    async fn get_capacity_by_lock_hash(
        &self,
        lock_hash: H256,
    ) -> RpcResult<indexer_legacy::LockHashCapacity>;

    #[method(name = "get_transactions_by_lock_hash")]
    async fn get_transactions_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> RpcResult<Vec<indexer_legacy::CellTransaction>>;
}

#[async_trait]
pub trait CkbRpc: Sync + Send + 'static {
    async fn local_node_info(&self) -> Result<LocalNode>;

    async fn get_tip_block_number(&self) -> Result<u64>;

    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool>;

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>>;

    async fn get_blocks_by_number(
        &self,
        block_number: Vec<BlockNumber>,
    ) -> Result<Vec<Option<BlockView>>>;

    async fn get_epoch_by_number(&self, epoch_number: Uint64) -> Result<EpochView>;

    async fn get_current_epoch(&self) -> Result<EpochView>;

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>>;
}

#[async_trait]
impl SyncAdapter for dyn CkbRpc {
    async fn pull_blocks(&self, block_numbers: Vec<BlockNumber>) -> Result<Vec<core::BlockView>> {
        let mut ret = Vec::new();
        for (idx, block) in self.get_blocks_by_number(block_numbers.clone()).await?.iter().enumerate() {
            if let Some(b) = block {
                ret.push(core::BlockView::from(b.to_owned()));
            } else {
                log::error!("[sync] Get none block {:?} from node", block_numbers[idx]);
            }
        }

        Ok(ret)
    }
}
