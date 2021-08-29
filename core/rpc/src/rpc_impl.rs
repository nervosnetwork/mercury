mod build_tx;
mod operation;
mod query;
mod transfer;
mod utils;

use crate::error::{RpcError, RpcErrorMessage, RpcResult};
use crate::rpc_impl::build_tx::calculate_tx_size_with_witness_placeholder;
use crate::types::{
    AddressOrLockHash, AdjustAccountPayload, AdvanceQueryPayload, AssetInfo, Balance, BlockInfo,
    DepositPayload, GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload,
    GetSpentTransactionPayload, GetTransactionInfoResponse, IOType, IdentityFlag, Item,
    MercuryInfo, QueryResponse, QueryTransactionsPayload, Record, SmartTransferPayload,
    TransactionCompletionResponse, TransactionStatus, TransferPayload, TxView, ViewType,
    WithdrawPayload,
};
use crate::{CkbRpc, MercuryRpcServer};

use common::anyhow::{anyhow, Result};
use common::utils::{parse_address, ScriptInfo};
use common::{
    hash::blake2b_160, Address, AddressPayload, CodeHashIndex, NetworkType, PaginationResponse,
    SECP256K1,
};
use core_storage::{DBAdapter, DBInfo, MercuryStore};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use ckb_jsonrpc_types::{TransactionView, TransactionWithStatus};
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use dashmap::DashMap;
use jsonrpsee_http_server::types::Error;
use parking_lot::RwLock;

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::{str::FromStr, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;
const INIT_ESTIMATE_FEE: u64 = BYTE_SHANNONS / 1000;
const DEFAULT_FEE_RATE: u64 = 1000;
const MAX_ITEM_NUM: usize = 1000;

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
    storage: MercuryStore<C>,
    builtin_scripts: HashMap<String, ScriptInfo>,
    ckb_client: C,
    network_type: NetworkType,
    cheque_timeout: RationalU256,
    cellbase_maturity: RationalU256,
}

#[async_trait]
impl<C: CkbRpc + DBAdapter> MercuryRpcServer for MercuryRpcImpl<C> {
    async fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse> {
        let item: Item = payload
            .item
            .try_into()
            .map_err(|e| Error::from(RpcError::from(e)))?;
        let tip_block_number = payload
            .tip_block_number
            .unwrap_or(self.storage.get_tip().await?.unwrap().0);
        let tip_epoch_number = self
            .get_epoch_by_number(tip_block_number)
            .await
            .map_err(|e| Error::from(RpcError::from(e)))?;

        let live_cells = self
            .get_live_cells_by_item(
                item.clone(),
                payload.asset_types.clone(),
                Some(tip_block_number),
                Some(tip_epoch_number.clone()),
                None,
                None,
            )
            .await
            .map_err(|e| Error::from(RpcError::from(e)))?;

        let mut balances_map: HashMap<(AddressOrLockHash, AssetInfo), Balance> = HashMap::new();

        let pubkey_hash = self
            .get_secp_lock_hash_by_item(item)
            .map_err(|e| Error::from(RpcError::from(e)))?;

        for cell in live_cells {
            let records = self
                .to_record(
                    &cell,
                    IOType::Output,
                    tip_block_number,
                    tip_epoch_number.clone(),
                )
                .await
                .map_err(|e| Error::from(RpcError::from(e)))?;

            // filter record, remain the one that owned by item.
            let records: Vec<Record> = records
                .into_iter()
                .filter(|record| {
                    match &record.address_or_lock_hash {
                        AddressOrLockHash::Address(address) => {
                            // unwrap here is ok, because if this address is invalid, it will throw error for more earlier.
                            let address = parse_address(address).unwrap();
                            match address.payload() {
                                AddressPayload::Short {
                                    net_ty: _,
                                    index: _,
                                    hash,
                                } => &pubkey_hash == hash,
                                AddressPayload::Full {
                                    hash_type: _,
                                    code_hash: _,
                                    args,
                                } => pubkey_hash == H160::from_slice(&args[0..20]).unwrap(),
                            }
                        }
                        AddressOrLockHash::LockHash(lock_hash) => {
                            pubkey_hash == H160::from_str(&lock_hash).unwrap()
                        }
                    }
                })
                .collect();

            self.accumulate_balance_from_records(&mut balances_map, &records, &tip_epoch_number)
                .await
                .map_err(|e| Error::from(RpcError::from(e)))?;
        }

        let balances = balances_map
            .into_iter()
            .map(|(_, balance)| balance)
            .collect();

        Ok(GetBalanceResponse {
            balances,
            tip_block_number,
        })
    }

    async fn get_block_info(&self, payload: GetBlockInfoPayload) -> RpcResult<BlockInfo> {
        let block_info = self
            .storage
            .get_simple_block(payload.block_hash, payload.block_number)
            .await;
        let block_info = match block_info {
            Ok(block_info) => block_info,
            Err(error) => {
                return Err(Error::from(RpcError::from(RpcErrorMessage::DBError(
                    error.to_string(),
                ))))
            }
        };
        let mut transactions = vec![];
        for tx_hash in block_info.transactions {
            let tx_info = self
                .get_transaction_info(tx_hash)
                .await
                .map(|res| res.transaction.expect("impossible: cannot find the tx"))?;
            transactions.push(tx_info);
        }
        Ok(BlockInfo {
            block_number: block_info.block_number,
            block_hash: block_info.block_hash,
            parent_hash: block_info.parent_hash,
            timestamp: block_info.timestamp,
            transactions,
        })
    }

    async fn get_transaction_info(&self, _tx_hash: H256) -> RpcResult<GetTransactionInfoResponse> {
        Ok(GetTransactionInfoResponse {
            transaction: None,
            status: TransactionStatus::Committed,
            reason: None,
        })
    }

    async fn query_transactions(
        &self,
        _payload: QueryTransactionsPayload,
    ) -> RpcResult<PaginationResponse<TxView>> {
        Ok(PaginationResponse {
            response: vec![],
            next_cursor: None,
            count: None,
        })
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
        if payload.from.is_empty() {
            return Err(Error::from(RpcError::from(
                RpcErrorMessage::NeedAtLeastOneFrom,
            )));
        }

        let mut estimate_fee = BYTE_SHANNONS;
        let fee_rate = payload.fee_rate.unwrap_or(BYTE_SHANNONS);

        loop {
            let response = self
                .build_deposit_transaction(payload.clone())
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
                    .get_secp_address_by_item(
                        payload.from[0]
                            .item
                            .clone()
                            .try_into()
                            .map_err(|e| Error::from(RpcError::from(e)))?,
                    )
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

    async fn build_withdraw_transaction(
        &self,
        _payload: WithdrawPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        Ok(TransactionCompletionResponse {
            tx_view: TransactionView::default(),
            sig_entries: vec![],
        })
    }

    async fn get_spent_transaction(
        &self,
        payload: GetSpentTransactionPayload,
    ) -> RpcResult<TxView> {
        match &payload.view_type {
            ViewType::TransactionView => self
                .get_spent_transaction_view(payload.outpoint)
                .await
                .map_err(|err| Error::from(RpcError::from(err))),
            ViewType::TransactionInfo => {
                let tx_hash = self
                    .storage
                    .get_spent_transaction_hash(payload.outpoint.into())
                    .await
                    .map_err(|error| {
                        Error::from(RpcError::from(RpcErrorMessage::DBError(error.to_string())))
                    })?;
                let tx_hash = match tx_hash {
                    Some(tx_hash) => tx_hash,
                    None => {
                        return Err(Error::from(RpcError::from(
                            RpcErrorMessage::CannotFindSpentTransaction,
                        )))
                    }
                };
                self.get_transaction_info(tx_hash).await.map(|res| {
                    TxView::TransactionInfo(
                        res.transaction.expect("impossible: cannot find the tx"),
                    )
                })
            }
        }
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
        storage: MercuryStore<C>,
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
    Address::from_str(addr).map_err(|e| anyhow!("{:?}", e))
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
