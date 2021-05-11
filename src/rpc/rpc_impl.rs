mod query;
mod rce_update;
mod transfer;

use crate::extensions::rce_validator::generated::xudt_rce;
use crate::extensions::{rce_validator, RCE_EXT_PREFIX};
use crate::rpc::types::{
    CreateWalletPayload, GetBalanceResponse, SMTUpdateItem, TransactionCompletionResponse,
    TransferPayload,
};
use crate::rpc::MercuryRpc;
use crate::utils::parse_address;
use crate::{stores::add_prefix, types::DeployedScriptConfig};

use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_jsonrpc_types::{Transaction, TransactionView};
use ckb_sdk::{AddressPayload, NetworkType};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256, U256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use log::debug;
use parking_lot::RwLock;

use std::collections::{HashMap, HashSet};
use std::{iter::Iterator, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    static ref ACP_USED_CACHE: DashMap<ThreadId, Vec<packed::OutPoint>> = DashMap::new();
}

use smt::{blake2b::Blake2bHasher, default_store::DefaultStore};

const MAX_RCE_RULE_NUM: usize = 8192;

pub type SMT = smt::SparseMerkleTree<Blake2bHasher, smt::H256, DefaultStore<smt::H256>>;

macro_rules! rpc_try {
    ($input: expr) => {
        $input.map_err(|e| Error::invalid_params(e.to_string()))?
    };
}

pub struct MercuryRpcImpl<S> {
    store: S,
    net_ty: NetworkType,
    _cheque_since: U256,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> MercuryRpc for MercuryRpcImpl<S>
where
    S: Store + Send + Sync + 'static,
{
    fn get_balance(&self, sudt_hash: Option<H256>, addr: String) -> RpcResult<GetBalanceResponse> {
        debug!("get udt {:?} balance address {:?}", sudt_hash, addr);
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.inner_get_balance(sudt_hash, &address));
        debug!("sudt balance {:?}", ret);
        Ok(ret)
    }

    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool> {
        let key = rce_validator::Key::Address(&rce_hash.pack(), &addr.pack()).into_vec();

        self.store
            .get(&add_prefix(*RCE_EXT_PREFIX, key))
            .map_or_else(
                |err| Err(Error::invalid_params(err.to_string())),
                |res| Ok(res.is_some()),
            )
    }

    fn transfer_with_rce_completion(&self, transaction: Transaction) -> RpcResult<TransactionView> {
        let tx: packed::Transaction = transaction.into();
        let mut witnesses = tx.witnesses().unpack();
        let tx_view = tx.clone().into_view();

        for (idx, (input, output)) in tx_view
            .inputs()
            .into_iter()
            .zip(tx_view.outputs().into_iter())
            .enumerate()
        {
            let input_detail = rpc_try!(self
                .get_detailed_live_cell(&input.previous_output())
                .map_err(|e| Error::invalid_params(e.to_string()))?
                .ok_or_else(|| MercuryError::CannotFindCellByOutPoint(
                    input.previous_output().into(),
                )));

            let input_hash: [u8; 32] = input_detail.cell_output.lock().calc_script_hash().unpack();
            let output_hash: [u8; 32] = output.lock().calc_script_hash().unpack();

            let xudt_data =
                xudt_rce::XudtData::from_slice(input_detail.cell_data.as_slice()).unwrap();

            let rules = rpc_try!(self.parse_xudt_data(xudt_data.data().unpack()));
            let proof = rpc_try!(self.check_rules(&[input_hash, output_hash], &rules));

            change_witness(&mut witnesses, idx, proof);
        }

        let w = witnesses
            .into_iter()
            .map(|bytes| bytes.pack())
            .collect::<Vec<packed::Bytes>>();

        Ok(tx.as_advanced_builder().set_witnesses(w).build().into())
    }

    // TODO: Support to update multiple rce script in one transaction
    fn rce_update_completion(
        &self,
        transaction: Transaction,
        update_items: Vec<SMTUpdateItem>,
    ) -> RpcResult<TransactionView> {
        let rce_pairs = rpc_try!(self.extract_rce_cells(&transaction));

        let rule = self.get_rc_rule(rce_pairs[0].input.cell_data.as_slice());
        let old_root: [u8; 32] = rule.smt_root().unpack();
        let mut smt = SMT::new(old_root.into(), DefaultStore::default());

        let proof = rpc_try!(self.build_proof(&smt, &update_items));
        rpc_try!(self.update_smt(&mut smt, &update_items));

        let new_root: [u8; 32] = smt.root().to_owned().into();
        let output_data = self.build_rce_data(new_root, rule.flags());
        let witness_args = rpc_try!(self.build_witness_args(proof, &update_items));

        Ok(self.build_rce_transaction(
            transaction.into(),
            rce_pairs[0].index,
            output_data,
            witness_args,
        ))
    }

    fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        debug!("transfer completion payload {:?}", payload);
        rpc_try!(payload.check());
        self.inner_transfer_complete(
            payload.udt_hash.clone(),
            payload.from.to_inner(),
            payload.to_inner_items(payload.udt_hash.is_some()),
            payload.change.clone(),
            payload.fee,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }

    fn build_wallet_creation_transaction(
        &self,
        payload: CreateWalletPayload,
    ) -> RpcResult<TransactionCompletionResponse> {
        debug!("create wallet payload {:?}", payload);
        self.inner_create_wallet(payload.ident, payload.info, payload.fee)
            .map_err(|e| Error::invalid_params(e.to_string()))
    }
}

impl<S: Store> MercuryRpcImpl<S> {
    pub fn new(
        store: S,
        net_ty: NetworkType,
        _cheque_since: U256,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        MercuryRpcImpl {
            store,
            net_ty,
            _cheque_since,
            config,
        }
    }
}

fn swap_item<T>(list: &mut [T], index: usize, new_item: T) {
    *list.get_mut(index).unwrap() = new_item;
}

fn change_witness(witnesses: &mut Vec<Bytes>, index: usize, witness_type_args: Bytes) {
    let witness_args = packed::WitnessArgs::from_slice(&witnesses[index]).unwrap();
    let new_witness = witness_args
        .as_builder()
        .input_type(Some(witness_type_args).pack())
        .build()
        .as_bytes();

    swap_item(witnesses, index, new_witness);
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
