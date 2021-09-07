mod build_tx;
mod consts;
mod operation;
mod query;
mod transfer;
mod utils;

pub use crate::rpc_impl::consts::{
    ckb, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, DEFAULT_FEE_RATE, INIT_ESTIMATE_FEE, MAX_ITEM_NUM,
    MIN_CKB_CAPACITY, STANDARD_SUDT_CAPACITY,
};

use crate::error::{RpcError, RpcErrorMessage, RpcResult};
use crate::rpc_impl::build_tx::calculate_tx_size_with_witness_placeholder;
use crate::types::{
    AddressOrLockHash, AdjustAccountPayload, AdvanceQueryPayload, AssetInfo, Balance, BlockInfo,
    DepositPayload, GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload,
    GetSpentTransactionPayload, GetTransactionInfoResponse, IOType, IdentityFlag, Item,
    MercuryInfo, QueryResponse, QueryTransactionsPayload, Record, SmartTransferPayload,
    StructureType, TransactionCompletionResponse, TransactionStatus, TransferPayload, TxView,
    ViewType, WithdrawPayload,
};
use crate::{CkbRpc, MercuryRpcServer};

use common::utils::{parse_address, ScriptInfo};
use common::{
    anyhow, hash::blake2b_160, Address, AddressPayload, CodeHashIndex, NetworkType,
    PaginationResponse, Result, SECP256K1,
};
use core_storage::{DBInfo, RelationalStorage, Storage};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use ckb_jsonrpc_types::{TransactionView, TransactionWithStatus};
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use dashmap::DashMap;
use jsonrpsee_http_server::types::Error;
use parking_lot::RwLock;

use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::{str::FromStr, thread::ThreadId};

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    pub static ref CURRENT_BLOCK_NUMBER: ArcSwap<BlockNumber> = ArcSwap::from_pointee(0u64);
    pub static ref CURRENT_EPOCH_NUMBER: ArcSwap<RationalU256> = ArcSwap::from_pointee(RationalU256::zero());
    static ref ACP_USED_CACHE: DashMap<ThreadId, Vec<packed::OutPoint>> = DashMap::new();
    static ref SECP256K1_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    static ref SUDT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    static ref ACP_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    static ref CHEQUE_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    static ref DAO_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
}

pub struct MercuryRpcImpl<C> {
    storage: RelationalStorage,
    builtin_scripts: HashMap<String, ScriptInfo>,
    ckb_client: C,
    network_type: NetworkType,
    cheque_timeout: RationalU256,
    cellbase_maturity: RationalU256,
}

#[async_trait]
impl<C: CkbRpc> MercuryRpcServer for MercuryRpcImpl<C> {
    async fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse> {
        self.inner_get_balance(payload)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn get_block_info(&self, payload: GetBlockInfoPayload) -> RpcResult<BlockInfo> {
        self.inner_get_block_info(payload)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn get_transaction_info(&self, tx_hash: H256) -> RpcResult<GetTransactionInfoResponse> {
        self.inner_get_transaction_info(tx_hash)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> RpcResult<PaginationResponse<TxView>> {
        self.inner_query_transaction(payload)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn build_adjust_account_transaction(
        &self,
        _payload: AdjustAccountPayload,
    ) -> RpcResult<Option<TransactionCompletionResponse>> {
        Ok(None)
    }

    async fn build_transfer_transaction(
        &self,
        _payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        Ok(TransactionCompletionResponse {
            tx_view: TransactionView::default(),
            sig_entries: vec![],
        })
    }

    async fn build_smart_transfer_transaction(
        &self,
        _payload: SmartTransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        Ok(TransactionCompletionResponse {
            tx_view: TransactionView::default(),
            sig_entries: vec![],
        })
    }

    async fn register_addresses(&self, addresses: Vec<String>) -> RpcResult<Vec<H160>> {
        let mut inputs: Vec<(H160, String)> = vec![];
        for addr_str in addresses {
            let address = match parse_address(&addr_str) {
                Ok(address) => address,
                Err(error) => {
                    return Err(Error::from(RpcError::from(RpcErrorMessage::CommonError(
                        error.to_string(),
                    ))))
                }
            };
            let lock = address_to_script(address.payload());
            let lock_hash = H160(blake2b_160(lock.as_slice()));
            inputs.push((lock_hash, addr_str));
        }
        self.inner_register_addresses(inputs)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    fn get_mercury_info(&self) -> RpcResult<MercuryInfo> {
        Ok(MercuryInfo {
            network_type: NetworkType::Testnet,
            mercury_version: Default::default(),
            ckb_node_version: Default::default(),
            enabled_extensions: vec![],
        })
    }

    fn get_db_info(&self) -> RpcResult<DBInfo> {
        self.inner_get_db_info()
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn build_deposit_transaction(
        &self,
        payload: DepositPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        self.inner_build_deposit_transaction(payload)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn build_withdraw_transaction(
        &self,
        payload: WithdrawPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        let item = match Item::try_from(payload.clone().from) {
            Ok(item) => item,
            Err(error) => return Err(Error::from(RpcError::from(error))),
        };
        let pay_item = match payload.clone().pay_fee {
            Some(pay_item) => {
                Item::try_from(pay_item).map_err(|err| Error::from(RpcError::from(err)))?
            }
            None => item.clone(),
        };

        let mut estimate_fee = INIT_ESTIMATE_FEE;
        let fee_rate = payload.fee_rate.unwrap_or(DEFAULT_FEE_RATE);

        loop {
            let response = self
                .inner_build_withdraw_transaction(item.clone(), pay_item.clone(), estimate_fee)
                .await
                .map_err(|e| Error::from(RpcError::from(e)))?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.sig_entries.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let change_address = self
                    .get_secp_address_by_item(pay_item)
                    .map_err(|e| Error::from(RpcError::from(e)))?;
                let tx_view = self
                    .update_tx_view_change_cell(
                        response.tx_view,
                        change_address,
                        estimate_fee,
                        actual_fee,
                    )
                    .map_err(|e| Error::from(RpcError::from(e)))?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.sig_entries);
                return Ok(adjust_response);
            }
        }
    }

    async fn get_spent_transaction(
        &self,
        payload: GetSpentTransactionPayload,
    ) -> RpcResult<TxView> {
        self.inner_get_spent_transaction(payload)
            .await
            .map_err(|err| Error::from(RpcError::from(err)))
    }

    async fn advance_query(
        &self,
        _payload: AdvanceQueryPayload,
    ) -> RpcResult<PaginationResponse<QueryResponse>> {
        Ok(PaginationResponse {
            response: vec![],
            next_cursor: None,
            count: None,
        })
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
    ) -> Self {
        MercuryRpcImpl {
            storage,
            builtin_scripts,
            ckb_client,
            network_type,
            cheque_timeout,
            cellbase_maturity,
        }
    }
}

pub fn address_to_script(payload: &AddressPayload) -> packed::Script {
    payload.into()
}

pub fn parse_normal_address(addr: &str) -> Result<Address> {
    Address::from_str(addr).map_err(|e| anyhow::anyhow!("{:?}", e))
}

pub fn pubkey_to_secp_address(lock_args: Bytes) -> H160 {
    let pubkey_hash = H160::from_slice(&lock_args[0..20]).unwrap();
    let script = packed::Script::from(&AddressPayload::new_short(
        NetworkType::Testnet,
        CodeHashIndex::Sighash,
        pubkey_hash,
    ));

    H160::from_slice(&blake2b_160(script.as_slice())).unwrap()
}

pub fn minstant_elapsed(start: u64) -> f64 {
    (minstant::now() - start) as f64 * minstant::nanos_per_cycle() / 1000f64
}
