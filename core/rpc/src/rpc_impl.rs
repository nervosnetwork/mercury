mod operation;
mod query;
mod transfer;

use crate::types::{
    CreateWalletPayload, GenericBlock, GetBalancePayload, GetBalanceResponse,
    GetGenericBlockPayload, GetGenericTransactionResponse, TransactionCompletionResponse,
    TransferPayload,
};
use crate::{error::RpcError, CkbRpc, MercuryRpc};

use common::anyhow::{anyhow, Result};
use common::{
    hash::blake2b_160, utils::parse_address, Address, AddressPayload, CodeHashIndex, MercuryError,
    NetworkType,
};
use core_extensions::{rce_validator, DeployedScriptConfig, RCE_EXT_PREFIX};
use core_storage::add_prefix;

use arc_swap::ArcSwap;
use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256, U256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use parking_lot::RwLock;

use std::collections::{HashMap, HashSet};
use std::{iter::Iterator, str::FromStr, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;
const INIT_ESTIMATE_FEE: u64 = BYTE_SHANNONS / 1000;

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    pub static ref USE_HEX_FORMAT: ArcSwap<bool> = ArcSwap::from_pointee(true);
    pub static ref CURRENT_BLOCK_NUMBER: ArcSwap<BlockNumber> = ArcSwap::from_pointee(0u64);
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


        rx.recv().unwrap()
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
    fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse> {
        log::debug!("get balance payload {:?}", payload);
        let ret = rpc_try!(self.inner_get_balance(
            payload.udt_hashes,
            payload.address,
            payload.block_number
        ));
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
        let is_udt = payload.udt_hash.is_some();
        self.inner_transfer_complete(
            payload.udt_hash.clone(),
            rpc_try!(self.handle_from_addresses(payload.from, is_udt)),
            rpc_try!(self.handle_to_items(payload.items, is_udt)),
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
        let address = rpc_try!(parse_key_address(&payload.key_address));
        self.inner_create_wallet(address, payload.info, payload.fee_rate)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn get_transaction_history(&self, ident: String) -> RpcResult<Vec<TransactionWithStatus>> {
        log::debug!("get transaction history ident {:?}", ident);
        self.inner_get_transaction_history(ident)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn register_addresses(&self, normal_addresses: Vec<String>) -> RpcResult<Vec<H160>> {
        log::debug!("register addresses {:?}", normal_addresses);
        self.inner_register_addresses(normal_addresses)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn get_generic_transaction(&self, tx_hash: H256) -> RpcResult<GetGenericTransactionResponse> {
        log::debug!("get generic transaction tx hash {:?}", tx_hash);
        let now = minstant::now();
        let tx = rpc_try!(block_on!(self, get_transactions, vec![tx_hash]))
            .get(0)
            .cloned()
            .unwrap()
            .unwrap();

        log::debug!("tx view {:?}, cost {}", tx, minstant_elapsed(now));
        let tx_hash = tx.transaction.hash;
        let tx_status = tx.tx_status.status;
        let (block_num, block_hash) =
            rpc_try!(self.get_tx_block_num_and_hash(tx_hash.0, tx_status.clone()));

        let confirmed_num = if let Some(num) = block_num {
            let current_num = **CURRENT_BLOCK_NUMBER.load();
            Some(current_num - num)
        } else {
            None
        };

        log::debug!(
            "block number {:?}, confirmed number {:?}, cost {}",
            block_num,
            confirmed_num,
            minstant_elapsed(now)
        );
        self.inner_get_generic_transaction(
            tx.transaction.inner.into(),
            tx_hash,
            tx_status,
            block_hash,
            block_num,
            confirmed_num,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn get_generic_block(&self, payload: GetGenericBlockPayload) -> RpcResult<GenericBlock> {
        let current_number = **CURRENT_BLOCK_NUMBER.load();
        let use_hex_format = **USE_HEX_FORMAT.load();

        let block = if payload.block_num.is_some() {
            let num = payload.block_num.unwrap();
            if num > current_number {
                return Err(Error::invalid_params("invalid block number"));
            }

            let resp = rpc_try!(block_on!(self, get_block_by_number, num, use_hex_format)).unwrap();
            if let Some(hash) = payload.block_hash {
                if resp.header.hash != hash {
                    return Err(Error::invalid_params("block number and hash mismatch"));
                }
            }
            resp
        } else if payload.block_hash.is_some() && payload.block_num.is_none() {
            let hash = payload.block_hash.unwrap();
            rpc_try!(block_on!(self, get_block, hash, use_hex_format)).unwrap()
        } else {
            rpc_try!(block_on!(
                self,
                get_block_by_number,
                current_number,
                use_hex_format
            ))
            .unwrap()
        };

        let block_num: u64 = block.header.inner.number.into();
        let txs = block
            .transactions
            .into_iter()
            .map(|tx| tx.inner.into())
            .collect::<Vec<packed::Transaction>>();

        self.inner_get_generic_block(
            txs,
            block_num,
            block.header.hash,
            block.header.inner.parent_hash,
            block.header.inner.timestamp.into(),
            current_number - block_num,
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

pub fn parse_key_address(addr: &str) -> Result<Address> {
    if Address::from_str(addr)
        .map_err(|e| anyhow!("{:?}", e))?
        .is_secp256k1()
    {
        parse_address(addr)
    } else {
        Err(MercuryError::rpc(RpcError::KeyAddressIsNotSecp256k1).into())
    }
}

pub fn parse_normal_address(addr: &str) -> Result<Address> {
    Address::from_str(addr).map_err(|e| anyhow!("{:?}", e))
}

pub fn pubkey_to_secp_address(lock_args: Bytes) -> H160 {
    let pubkey_hash = H160::from_slice(&lock_args[0..20]).unwrap();
    let script = packed::Script::from(&AddressPayload::new_short(
        CodeHashIndex::Sighash,
        pubkey_hash,
    ));

    H160::from_slice(&blake2b_160(script.as_slice())).unwrap()
}

pub fn minstant_elapsed(start: u64) -> f64 {
    (minstant::now() - start) as f64 * minstant::nanos_per_cycle() / 1000f64
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
