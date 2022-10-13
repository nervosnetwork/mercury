use crate::{lock_extension::LockScriptHandler, RelationalStorage};

use ckb_jsonrpc_types::CellDep;
use common::lazy::{DAO_CODE_HASH, SUDT_CODE_HASH};
use common::{utils::decode_udt_amount, NetworkType, Result};
use core_rpc_types::Identity;

use ckb_types::bytes;
use ckb_types::core::RationalU256;
use ckb_types::core::ScriptHashType;
use ckb_types::packed::{Bytes, Script, ScriptOpt};
use ckb_types::prelude::*;
use ckb_types::H256;

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

inventory::submit!(LockScriptHandler {
    name: "omni_lock",
    get_name,
    get_code_hash,
    query_tip,
    is_occupied_free,
    query_lock_scripts_by_identity,
});

fn get_name() -> String {
    "omni_lock".into()
}

fn get_code_hash(network: NetworkType) -> H256 {
    match network {
        NetworkType::Mainnet => {
            H256::from_str("9b819793a64463aed77c615d6cb226eea5487ccfc0783043a587254cda2b6f26")
                .unwrap()
        }
        NetworkType::Testnet => {
            H256::from_str("f329effd1c475a2978453c8600e1eaf0bc2087ee093c3ee64cc96ec6847752cb")
                .unwrap()
        }
        NetworkType::Staging => unreachable!(),
        NetworkType::Dev => unreachable!(),
    }
}

fn _get_hash_type() -> ScriptHashType {
    ScriptHashType::Type
}

fn _get_cell_dep() -> CellDep {
    todo!()
}

fn _get_live_cell_priority() -> u32 {
    5
}

fn is_occupied_free(_lock_args: &Bytes, cell_type: &ScriptOpt, cell_data: &bytes::Bytes) -> bool {
    if cell_data.is_empty() && cell_type.is_none() {
        return true;
    }

    if let Some(type_script) = cell_type.to_opt() {
        let type_code_hash: H256 = type_script.code_hash().unpack();
        // a secp sUDT cell with 0 udt amount should be spendable.
        if Some(&type_code_hash) == SUDT_CODE_HASH.get() && decode_udt_amount(cell_data) == Some(0)
        {
            // to do refactoring: SUDT_CODE_HASH can be get from config file?
            return true;
        }
        if Some(&type_code_hash) == DAO_CODE_HASH.get() {
            todo!()
        }
    }

    false
}

fn _is_unlock(_from: RationalU256, _end: Option<RationalU256>) -> bool {
    todo!()
}

fn _is_anyone_can_pay(_lock_args: Option<Bytes>) -> bool {
    todo!()
}

fn _address_to_identity(_address: &str) -> Result<Identity> {
    todo!()
}

fn query_tip(
    storage: &'_ RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<u64>> // future API / pollable
            + Send // required by non-single-threaded executors
            + '_,
    >,
> {
    Box::pin(storage.get_tip_number())
}

fn query_lock_scripts_by_identity(
    _identity: Identity,
    _storage: &'_ RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<Vec<Script>>> // future API / pollable
            + Send // required by non-single-threaded executors
            + '_,
    >,
> {
    todo!()
}
