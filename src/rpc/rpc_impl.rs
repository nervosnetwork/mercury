mod query;
mod transfer;

use crate::extensions::{rce_validator, RCE_EXT_PREFIX};
use crate::rpc::types::{CreateWalletPayload, TransferCompletionResponse, TransferPayload};
use crate::rpc::MercuryRpc;
use crate::utils::parse_address;
use crate::{stores::add_prefix, types::DeployedScriptConfig};

use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_sdk::AddressPayload;
use ckb_types::{packed, prelude::*, H256, U256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use log::debug;

use std::{collections::HashMap, iter::Iterator, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;

lazy_static::lazy_static! {
    static ref ACP_USED_CACHE: DashMap<ThreadId, Vec<packed::OutPoint>> = DashMap::new();
}

macro_rules! rpc_try {
    ($input: expr) => {
        $input.map_err(|e| Error::invalid_params(e.to_string()))?
    };
}

pub struct MercuryRpcImpl<S> {
    store: S,
    _cheque_since: U256,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> MercuryRpc for MercuryRpcImpl<S>
where
    S: Store + Send + Sync + 'static,
{
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>> {
        debug!("get ckb balance address {:?}", addr);
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.ckb_balance(&address));
        debug!("ckb balance {:?}", ret);
        Ok(ret)
    }

    // Todo: u128 cannot be serialize by serde_json
    fn get_sudt_balance(&self, sudt_hash: H256, addr: String) -> RpcResult<Option<u64>> {
        debug!("get sudt {:?} balance address {:?}", sudt_hash, addr);
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(&address, sudt_hash));
        debug!("sudt balance {:?}", ret);
        Ok(ret.map(|b| b as u64))
    }

    fn get_xudt_balance(&self, xudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(&address, xudt_hash));
        Ok(ret)
    }

    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool> {
        let key = rce_validator::Key::Address(&rce_hash.pack(), &addr.pack()).into_vec();

        self.store
            .get(&add_prefix(*RCE_EXT_PREFIX, key))
            .map_or_else(|_| Err(Error::internal_error()), |res| Ok(res.is_some()))
    }

    fn transfer_completion(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransferCompletionResponse> {
        debug!("transfer completion payload {:?}", payload);
        self.inner_transfer_complete(
            payload.udt_hash.clone(),
            payload.from.to_inner(),
            payload.to_inner_items(),
            payload.change.clone(),
            payload.fee,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn create_wallet(&self, payload: CreateWalletPayload) -> RpcResult<TransferCompletionResponse> {
        debug!("create wallet payload {:?}", payload);
        self.inner_create_wallet(payload.address, payload.info, payload.fee)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }
}

impl<S: Store> MercuryRpcImpl<S> {
    pub fn new(
        store: S,
        _cheque_since: U256,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        MercuryRpcImpl {
            store,
            _cheque_since,
            config,
        }
    }
}

fn address_to_script(payload: &AddressPayload) -> packed::Script {
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
