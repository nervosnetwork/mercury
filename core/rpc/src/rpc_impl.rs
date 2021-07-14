mod query;
mod transfer;

use crate::types::{
    CreateWalletPayload, GetBalancePayload, GetBalanceResponse, ScanBlockPayload,
    ScanBlockResponse, TransactionCompletionResponse, TransferPayload,
};
use crate::{CkbRpc, MercuryRpc};

use common::{utils::parse_address, AddressPayload, NetworkType};
use core_extensions::{rce_validator, DeployedScriptConfig, RCE_EXT_PREFIX};
use core_storage::add_prefix;

use arc_swap::ArcSwap;
use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::{core::RationalU256, packed, prelude::*, H256, U256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use parking_lot::RwLock;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::{iter::Iterator, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;
const INIT_ESTIMATE_FEE: u64 = BYTE_SHANNONS / 1000;

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    pub static ref USE_HEX_FORMAT: ArcSwap<bool> = ArcSwap::new(Arc::new(true));
    static ref ACP_USED_CACHE: DashMap<ThreadId, Vec<packed::OutPoint>> = DashMap::new();
}

#[macro_export]
macro_rules! block_on {
    ($self_: ident, $func: ident $(, $arg: expr)*) => {{
        use jsonrpc_http_server::tokio::runtime;

        let (tx, rx) = crossbeam_channel::bounded(1);
        let client_clone = $self_.ckb_client.clone();

        std::thread::spawn(move || {
            let mut rt = runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let res = client_clone.$func($($arg),*).await;
                tx.send(res).unwrap();
            })
        });


        rx.recv()
            .map_err(|e| MercuryError::rpc(RpcError::ChannelError(e.to_string())))?
    }};
}

macro_rules! rpc_try {
    ($input: expr) => {
        $input.map_err(|e| Error::invalid_params(e.to_string()))?
    };
}

pub struct MercuryRpcImpl<S, C> {
    store: S,
    net_ty: NetworkType,
    ckb_client: C,
    cheque_since: RationalU256,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S, C> MercuryRpc for MercuryRpcImpl<S, C>
where
    S: Store + Send + Sync + 'static,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<Vec<GetBalanceResponse>> {
        log::debug!("get balance payload {:?}", payload);
        let address = rpc_try!(parse_address(&payload.address));
        let ret = rpc_try!(self.inner_get_balance(payload.udt_hashes, &address));
        log::debug!("sudt balance {:?}", ret);
        Ok(ret)
    }

    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool> {
        let key = rce_validator::Key::Address(&rce_hash.pack(), &addr.pack()).into_vec();

        self.store
            .get(&add_prefix(*RCE_EXT_PREFIX, key))
            .map_or_else(|_| Err(Error::internal_error()), |res| Ok(res.is_some()))
    }

    fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        log::debug!("transfer completion payload {:?}", payload);
        rpc_try!(payload.check());
        self.inner_transfer_complete(
            payload.udt_hash.clone(),
            payload.from.to_inner(),
            payload.to_inner_items(payload.udt_hash.is_some()),
            payload.change.clone(),
            payload.fee_rate,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn build_wallet_creation_transaction(
        &self,
        payload: CreateWalletPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        log::debug!("create wallet payload {:?}", payload);
        self.inner_create_wallet(payload.ident, payload.info, payload.fee_rate)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn get_transaction_history(&self, ident: String) -> RpcResult<Vec<TransactionWithStatus>> {
        log::debug!("get transaction history ident {:?}", ident);
        self.inner_get_transaction_history(ident)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn scan_deposit(&self, payload: ScanBlockPayload) -> RpcResult<ScanBlockResponse> {
        log::debug!("query charge payload {:?}", payload);
        self.inner_scan_deposit(
            payload.block_number,
            payload.udt_hash.clone(),
            payload.idents,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }
}

impl<S: Store, C: CkbRpc> MercuryRpcImpl<S, C> {
    pub fn new(
        store: S,
        net_ty: NetworkType,
        ckb_client: C,
        cheque_since: U256,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        let cheque_since = RationalU256::from_u256(cheque_since);
        MercuryRpcImpl {
            store,
            net_ty,
            ckb_client,
            cheque_since,
            config,
        }
    }
}

pub fn address_to_script(payload: &AddressPayload) -> packed::Script {
    payload.into()
}

fn udt_iter(
    input: &[(DetailedLiveCell, packed::OutPoint)],
    hash: packed::Byte32,
) -> impl Iterator<Item = &(DetailedLiveCell, packed::OutPoint)> {
    input.iter().filter(move |(cell, _)| {
        if let Some(script) = cell.cell_output.type_().to_opt() {
            script.calc_script_hash() == hash
        } else {
            false
        }
    })
}

fn ckb_iter(
    cells: &[(DetailedLiveCell, packed::OutPoint)],
) -> impl Iterator<Item = &(DetailedLiveCell, packed::OutPoint)> {
    cells
        .iter()
        .filter(|(cell, _)| cell.cell_output.type_().is_none())
}
