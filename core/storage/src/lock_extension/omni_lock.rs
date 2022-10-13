use crate::{lock_extension::LockScriptHandler, RelationalStorage};

use ckb_jsonrpc_types::CellDep;
use common::utils::ScriptInfo;
use common::{NetworkType, Result};
use core_rpc_types::Identity;

use ckb_types::core::{RationalU256, ScriptHashType};
use ckb_types::packed::{Bytes, Script, ScriptOpt};
use ckb_types::H256;

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

// impl LockScript for LockScriptHandler {
//     fn get_lock_info(&self) -> ScriptInfo {
//         todo!()
//     }

//     fn caculate_occupied(&self, lock_args: &Bytes, type_: &ScriptOpt, data: &Bytes) -> u64 {
//         todo!()
//     }

//     fn is_unlock(&self, from: RationalU256, end: Option<RationalU256>) -> bool {
//         todo!()
//     }

//     fn is_anyone_can_pay(&self, lock_args: Option<Bytes>) -> bool {
//         todo!()
//     }

//     fn address_to_identity(&self, address: &str) -> Result<Identity> {
//         todo!()
//     }
// }

inventory::submit!(LockScriptHandler {
    name: "omni_lock",
    get_name,
    get_code_hash,
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

fn get_hash_type() -> ScriptHashType {
    ScriptHashType::Type
}

fn get_cell_dep() -> CellDep {
    todo!()
}

fn get_live_cell_priority() -> u32 {
    5
}

fn query_lock_scripts_by_identity(
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
