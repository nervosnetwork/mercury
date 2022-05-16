mod adjust_account;
mod build_tx;
mod operation;
mod query;
pub(crate) mod utils;
pub(crate) mod utils_types;

use crate::r#impl::build_tx::calculate_tx_size;
use crate::{error::CoreError, MercuryRpcServer};

use ckb_jsonrpc_types::Uint64;
use ckb_types::core::RationalU256;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use clap::crate_version;
use common::utils::ScriptInfo;
use common::{
    hash::blake2b_160, Address, AddressPayload, Context, NetworkType, PaginationResponse, ACP,
    CHEQUE, DAO, PW_LOCK, SECP256K1, SUDT,
};
use core_ckb_client::CkbRpc;
use core_rpc_types::error::MercuryRpcError;
use core_rpc_types::lazy::{
    ACP_CODE_HASH, CHEQUE_CODE_HASH, DAO_CODE_HASH, PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH,
    SUDT_CODE_HASH,
};
use core_rpc_types::{
    indexer, AdjustAccountPayload, BlockInfo, DaoClaimPayload, DaoDepositPayload,
    DaoWithdrawPayload, GetAccountInfoPayload, GetAccountInfoResponse, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, MercuryInfo, QueryTransactionsPayload, SimpleTransferPayload,
    SudtIssuePayload, SyncState, TransactionCompletionResponse, TransferPayload, TxView,
};
use core_storage::{DBInfo, RelationalStorage};
use jsonrpsee_core::{async_trait, Error, RpcResult};
use parking_lot::RwLock;
use pprof::ProfilerGuard;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref PROFILER_GUARD: std::sync::Mutex<Option<ProfilerGuard<'static>>> = std::sync::Mutex::new(None);
}

macro_rules! rpc_impl {
    ($self_: ident, $func: ident, $payload: expr) => {{
        let (_, collector) = common_logger::Span::root("trace_name");
        let _collector = common_logger::MercuryTrace::new(collector);

        $self_
            .$func(Context::new(), $payload)
            .await
            .map_err(Into::into)
    }};
}

pub struct MercuryRpcImpl<C> {
    storage: RelationalStorage,
    builtin_scripts: HashMap<String, ScriptInfo>,
    ckb_client: C,
    network_type: NetworkType,
    cheque_timeout: RationalU256,
    cellbase_maturity: RationalU256,
    sync_state: Arc<RwLock<SyncState>>,
    pool_cache_size: u64,
    is_pprof_enabled: bool,
}

#[async_trait]
impl<C: CkbRpc> MercuryRpcServer for MercuryRpcImpl<C> {
    async fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse> {
        rpc_impl!(self, inner_get_balance, payload)
    }

    async fn get_block_info(&self, payload: GetBlockInfoPayload) -> RpcResult<BlockInfo> {
        rpc_impl!(self, inner_get_block_info, payload)
    }

    async fn get_transaction_info(&self, tx_hash: H256) -> RpcResult<GetTransactionInfoResponse> {
        rpc_impl!(self, inner_get_transaction_info, tx_hash)
    }

    async fn query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> RpcResult<PaginationResponse<TxView>> {
        rpc_impl!(self, inner_query_transactions, payload)
    }

    async fn get_account_info(
        &self,
        payload: GetAccountInfoPayload,
    ) -> RpcResult<GetAccountInfoResponse> {
        rpc_impl!(self, inner_get_account_info, payload)
    }

    async fn build_adjust_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> RpcResult<Option<TransactionCompletionResponse>> {
        rpc_impl!(self, inner_build_adjust_account_transaction, payload)
    }

    async fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_transfer_transaction, payload)
    }

    async fn build_simple_transfer_transaction(
        &self,
        payload: SimpleTransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_simple_transfer_transaction, payload)
    }

    async fn register_addresses(&self, addresses: Vec<String>) -> RpcResult<Vec<H160>> {
        let mut inputs: Vec<(H160, String)> = vec![];
        for addr_str in addresses {
            let address = Address::from_str(&addr_str)
                .map_err(|e| MercuryRpcError::from(CoreError::ParseAddressError(e)))?;
            let lock = address_to_script(address.payload());
            let lock_hash = H160(blake2b_160(lock.as_slice()));
            inputs.push((lock_hash, addr_str));
        }

        rpc_impl!(self, inner_register_addresses, inputs)
    }

    fn get_mercury_info(&self) -> RpcResult<MercuryInfo> {
        Ok(MercuryInfo {
            network_type: self.network_type,
            mercury_version: crate_version!().to_string(),
            ckb_node_version: "v0.101".to_string(),
            enabled_extensions: vec![],
        })
    }

    fn get_db_info(&self) -> RpcResult<DBInfo> {
        self.inner_get_db_info(Context::new()).map_err(Into::into)
    }

    async fn build_dao_deposit_transaction(
        &self,
        payload: DaoDepositPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_dao_deposit_transaction, payload)
    }

    async fn build_dao_withdraw_transaction(
        &self,
        payload: DaoWithdrawPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_dao_withdraw_transaction, payload)
    }

    async fn build_dao_claim_transaction(
        &self,
        payload: DaoClaimPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_dao_claim_transaction, payload)
    }

    async fn build_sudt_issue_transaction(
        &self,
        payload: SudtIssuePayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        rpc_impl!(self, inner_build_sudt_issue_transaction, payload)
    }

    async fn get_spent_transaction(
        &self,
        payload: GetSpentTransactionPayload,
    ) -> RpcResult<TxView> {
        rpc_impl!(self, inner_get_spent_transaction, payload)
    }

    async fn get_tip(&self) -> RpcResult<Option<indexer::Tip>> {
        self.inner_get_tip(Context::new()).await.map_err(Into::into)
    }

    async fn get_cells(
        &self,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
    ) -> RpcResult<indexer::PaginationResponse<indexer::Cell>> {
        self.inner_get_cells(Context::new(), search_key, order, limit, after_cursor)
            .await
            .map_err(Into::into)
    }

    async fn get_cells_capacity(
        &self,
        search_key: indexer::SearchKey,
    ) -> RpcResult<indexer::CellsCapacity> {
        self.inner_get_cells_capacity(Context::new(), search_key)
            .await
            .map_err(Into::into)
    }

    async fn get_transactions(
        &self,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
    ) -> RpcResult<indexer::PaginationResponse<indexer::Transaction>> {
        self.inner_get_transaction(Context::new(), search_key, order, limit, after_cursor)
            .await
            .map_err(Into::into)
    }

    async fn get_ckb_uri(&self) -> RpcResult<Vec<String>> {
        let res = self
            .ckb_client
            .local_node_info()
            .await?
            .addresses
            .iter()
            .map(|addr| addr.address.clone())
            .collect::<Vec<_>>();
        Ok(res)
    }

    async fn get_live_cells_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> RpcResult<Vec<indexer::LiveCell>> {
        self.inner_get_live_cells_by_lock_hash(
            Context::new(),
            lock_hash,
            page,
            per_page,
            reverse_order,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_capacity_by_lock_hash(
        &self,
        lock_hash: H256,
    ) -> RpcResult<indexer::LockHashCapacity> {
        self.inner_get_capacity_by_lock_hash(Context::new(), lock_hash)
            .await
            .map_err(Into::into)
    }

    async fn get_transactions_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> RpcResult<Vec<indexer::CellTransaction>> {
        self.inner_get_transactions_by_lock_hash(
            Context::new(),
            lock_hash,
            page,
            per_page,
            reverse_order,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_sync_state(&self) -> RpcResult<SyncState> {
        self.inner_get_sync_state(Context::new())
            .await
            .map_err(Into::into)
    }

    async fn start_profiler(&self) -> RpcResult<()> {
        if !self.is_pprof_enabled {
            return Err(Error::MethodNotFound("start_profiler".to_string()));
        }
        log::info!("profiler started");
        let profiler_guard = ProfilerGuard::new(100).map_err(|e| Error::Custom(e.to_string()))?;
        let mut lock = PROFILER_GUARD
            .try_lock()
            .map_err(|e| Error::Custom(e.to_string()))?;
        *lock = Some(profiler_guard);
        Ok(())
    }

    async fn report_pprof(&self) -> RpcResult<()> {
        if !self.is_pprof_enabled {
            return Err(Error::MethodNotFound("report_pprof".to_string()));
        }
        log::info!("profiler started");
        let mut profiler = PROFILER_GUARD
            .try_lock()
            .map_err(|e| Error::Custom(e.to_string()))?;
        if let Some(profiler) = profiler.take() {
            if let Ok(report) = profiler.report().build() {
                let file = std::fs::File::create("./free-space/flamegraph.svg")?;
                let mut options = pprof::flamegraph::Options::default();
                options.image_width = Some(2500);
                report
                    .flamegraph_with_options(file, &mut options)
                    .map_err(|e| Error::Custom(e.to_string()))?;
            }
        }
        Ok(())
    }
}

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub fn new(
        storage: RelationalStorage,
        builtin_scripts: HashMap<String, ScriptInfo>,
        ckb_client: C,
        network_type: NetworkType,
        cheque_timeout: RationalU256,
        cellbase_maturity: RationalU256,
        sync_state: Arc<RwLock<SyncState>>,
        pool_cache_size: u64,
        is_pprof_enabled: bool,
    ) -> Self {
        SECP256K1_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(SECP256K1)
                .cloned()
                .expect("get secp256k1 script info")
                .script
                .code_hash()
                .unpack(),
        ));
        SUDT_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(SUDT)
                .cloned()
                .expect("get sudt script info")
                .script
                .code_hash()
                .unpack(),
        ));
        ACP_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(ACP)
                .cloned()
                .expect("get acp script info")
                .script
                .code_hash()
                .unpack(),
        ));
        CHEQUE_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(CHEQUE)
                .cloned()
                .expect("get cheque script info")
                .script
                .code_hash()
                .unpack(),
        ));
        DAO_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(DAO)
                .cloned()
                .expect("get dao script info")
                .script
                .code_hash()
                .unpack(),
        ));
        PW_LOCK_CODE_HASH.swap(Arc::new(
            builtin_scripts
                .get(PW_LOCK)
                .cloned()
                .expect("get pw lock script info")
                .script
                .code_hash()
                .unpack(),
        ));

        MercuryRpcImpl {
            storage,
            builtin_scripts,
            ckb_client,
            network_type,
            cheque_timeout,
            cellbase_maturity,
            sync_state,
            pool_cache_size,
            is_pprof_enabled,
        }
    }
}

pub fn address_to_script(payload: &AddressPayload) -> packed::Script {
    payload.into()
}
