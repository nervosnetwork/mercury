mod operation;
mod query;
mod transfer;

use crate::{error::RpcError, CkbRpc};

use common::anyhow::{anyhow, Result};
use common::{
    hash::blake2b_160, utils::parse_address, Address, AddressPayload, CodeHashIndex, MercuryError,
    NetworkType, Order,
};

use arc_swap::ArcSwap;
use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_types::core::BlockNumber;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256, U256};
use dashmap::DashMap;
use parking_lot::RwLock;

use std::collections::HashSet;
use std::{iter::Iterator, str::FromStr, thread::ThreadId};

pub const BYTE_SHANNONS: u64 = 100_000_000;
pub const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;
pub const CHEQUE_CELL_CAPACITY: u64 = 162 * BYTE_SHANNONS;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;
const INIT_ESTIMATE_FEE: u64 = BYTE_SHANNONS / 1000;
const DEFAULT_FEE_RATE: u64 = 1000;
const MAX_ITEM_NUM: usize = 1000;

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
            let rt = runtime::Runtime::new().unwrap();

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
        .filter(|(cell, _)| cell.cell_output.type_().is_none() && cell.cell_data.is_empty())
}
