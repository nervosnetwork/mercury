use crate::{lock_extension::LockScriptHandler, RelationalStorage};
use crate::{DetailedCell, Storage};

pub use ckb_sdk::types::omni_lock::OmniLockWitnessLock;

use ckb_jsonrpc_types::CellDep;
use common::lazy::{DAO_CODE_HASH, EXTENSION_LOCK_SCRIPT_INFOS, SUDT_CODE_HASH};
use common::{utils::decode_udt_amount, Result, SECP256K1};
use core_rpc_types::{ExtraFilter, Identity, ScriptGroup};

use ckb_types::bytes;
use ckb_types::core::RationalU256;
use ckb_types::core::ScriptHashType;
use ckb_types::packed::{Bytes, BytesOpt, Script, ScriptOpt};
use ckb_types::prelude::*;
use ckb_types::{H160, H256};

use std::collections::BTreeSet;

#[macro_export]
macro_rules! dyn_async {(
    $( #[$attr:meta] )* // includes doc strings
    $pub:vis
    async
    fn $fname:ident<$lt:lifetime> ( $($args:tt)* ) $(-> $Ret:ty)?
    {
        $($body:tt)*
    }
) => (
    $( #[$attr] )*
    #[allow(unused_parens)]
    $pub
    fn $fname<$lt> ( $($args)* ) -> ::std::pin::Pin<::std::boxed::Box<
        dyn $lt + Send + ::std::future::Future<Output = ($($Ret)?)>
    >>
    {
        Box::pin(async move { $($body)* })
    }
)}

inventory::submit!(LockScriptHandler {
    name: "omni_lock",
    is_occupied_free,
    query_lock_scripts_by_identity,
    generate_extra_filter,
    script_to_identity,
    can_be_pooled_ckb,
    get_witness_lock_placeholder,
    insert_script_deps,
});

fn _get_hash_type() -> ScriptHashType {
    ScriptHashType::Type
}

fn _get_cell_dep() -> CellDep {
    todo!()
}

fn _get_live_cell_priority() -> u32 {
    5
}

fn can_be_pooled_ckb() -> bool {
    true
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
            return true;
        }
        if Some(&type_code_hash) == DAO_CODE_HASH.get() {
            todo!()
        }
    }

    false
}

fn generate_extra_filter(type_script: Script) -> Option<ExtraFilter> {
    let type_code_hash: H256 = type_script.code_hash().unpack();
    if Some(&type_code_hash) == SUDT_CODE_HASH.get() {
        None
    } else {
        Some(ExtraFilter::Frozen)
    }
}

fn _is_unlock(_from: RationalU256, _end: Option<RationalU256>) -> bool {
    todo!()
}

fn _is_anyone_can_pay(_lock_args: Option<Bytes>) -> bool {
    todo!()
}

dyn_async! {
    async fn query_lock_scripts_by_identity<'a>(
        self_: &'a LockScriptHandler,
        identity: &'a Identity,
        storage: &'a RelationalStorage,
    ) -> Result<Vec<Script>> {
        let mut ret = vec![];
        if let Some(extension_infos) = EXTENSION_LOCK_SCRIPT_INFOS.get() {
            if let Some(info) = extension_infos.get(self_.name) {
                let code_hash: H256 = info.script.code_hash().unpack();
                let mut scripts = storage
                    .get_scripts_by_partial_arg(
                        &code_hash,
                        bytes::Bytes::from(identity.0.to_vec()),
                        (0, 21),
                    )
                    .await?;
                ret.append(&mut scripts)
            }
        }
        Ok(ret)
    }
}

fn script_to_identity(self_: &LockScriptHandler, script: &Script) -> Option<Identity> {
    let extension_infos = EXTENSION_LOCK_SCRIPT_INFOS.get()?;
    let info = extension_infos.get(self_.name)?;
    if info.script.code_hash() == script.code_hash() {
        let flag = script.args().as_slice()[4..5].to_vec()[0].try_into().ok()?;
        let hash = H160::from_slice(&script.args().as_slice()[5..25]).ok()?;
        return Some(Identity::new(flag, hash));
    }
    None
}

fn insert_script_deps(cell: &DetailedCell, script_deps: &mut BTreeSet<String>) {
    if let Some(lock_handler) = cell.lock_handler {
        script_deps.insert(lock_handler.name.to_string());
    }
    script_deps.insert(SECP256K1.to_string());
}

fn get_witness_lock_placeholder(_script_group: &ScriptGroup) -> BytesOpt {
    let witness_lock = OmniLockWitnessLock::new_builder()
        .signature(Some(bytes::Bytes::from(vec![0u8; 65])).pack())
        .build();
    Some(witness_lock.as_bytes()).pack()
}
