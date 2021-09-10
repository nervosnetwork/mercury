#![allow(
    clippy::mutable_key_type,
    clippy::upper_case_acronyms,
    unused_imports,
    dead_code
)]

pub mod ckb_client;
pub mod rpc_impl;
pub mod types;
pub mod indexer_types;

mod error;
#[cfg(test)]
mod tests;

use error::{RpcErrorMessage, RpcResult};
use types::{
    AdjustAccountPayload, AdvanceQueryPayload, BlockInfo, DepositPayload, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, MercuryInfo, QueryResponse, QueryTransactionsPayload,
    SmartTransferPayload, TransactionCompletionResponse, TransferPayload, TxView, WithdrawPayload,
};
use indexer_types:: {
    GetCellPayload
};

pub use ckb_client::CkbRpcClient;
pub use rpc_impl::{MercuryRpcImpl, CURRENT_BLOCK_NUMBER, TX_POOL_CACHE};

use common::{PaginationResponse, Result};
use core_storage::DBInfo;
use core_synchronization::SyncAdapter;

use async_trait::async_trait;
use ckb_jsonrpc_types::{BlockView, EpochView, LocalNode, RawTxPool, TransactionWithStatus};
use ckb_types::{core, core::BlockNumber, H160, H256};
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

    #[method(name = "get_cells")]
    async fn get_cells(
        &self,
        payload: GetCellPayload,
    ) -> RpcResult<indexer_types::PaginationResponse<indexer_types::Cell>>;
}

#[async_trait]
pub trait CkbRpc: Sync + Send + 'static {
    async fn local_node_info(&self) -> Result<LocalNode>;

    async fn get_tip_block_number(&self) -> Result<BlockNumber>;

    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool>;

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>>;

    async fn get_blocks_by_number(
        &self,
        block_number: Vec<BlockNumber>,
    ) -> Result<Vec<Option<BlockView>>>;

    async fn get_epoch_by_number(&self, epoch_number: u64) -> Result<EpochView>;

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>>;
}

#[async_trait]
impl SyncAdapter for dyn CkbRpc {
    async fn pull_blocks(&self, block_numbers: Vec<BlockNumber>) -> Result<Vec<core::BlockView>> {
        let mut ret = Vec::new();
        for block in self.get_blocks_by_number(block_numbers).await?.iter() {
            if let Some(b) = block {
                ret.push(core::BlockView::from(b.to_owned()));
            } else {
                return Err(RpcErrorMessage::GetNoneBlockFromNode.into());
            }
        }

        Ok(ret)
    }
}
