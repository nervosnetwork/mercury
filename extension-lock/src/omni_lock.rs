use crate::LockScriptHandler;

pub use ckb_sdk::types::omni_lock::OmniLockWitnessLock;

use common::lazy::{DAO_CODE_HASH, SUDT_CODE_HASH};
use common::{utils::decode_udt_amount, Result, SECP256K1};
use core_rpc_types::{ExtraFilter, Identity, ScriptGroup};
use core_storage::RelationalStorage;
use core_storage::Storage;

use ckb_jsonrpc_types::CellDep;
use ckb_sdk::unlock::OmniLockConfig;
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
    get_acp_script,
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
        code_hash: &'a H256,
        identity: &'a Identity,
        storage: &'a RelationalStorage,
    ) -> Result<Vec<Script>> {
        storage
            .get_scripts_by_partial_arg(code_hash, bytes::Bytes::from(identity.0.to_vec()), (0, 21))
            .await
    }
}

fn script_to_identity(script: &Script) -> Option<Identity> {
    let flag = script.args().raw_data()[0].try_into().ok()?;
    let hash = H160::from_slice(&script.args().raw_data()[1..21]).ok()?;
    Some(Identity::new(flag, hash))
}

fn insert_script_deps(lock_name: &str, script_deps: &mut BTreeSet<String>) {
    script_deps.insert(lock_name.to_string());
    script_deps.insert(SECP256K1.to_string());
}

fn get_witness_lock_placeholder(_script_group: &ScriptGroup) -> BytesOpt {
    let witness_lock = OmniLockWitnessLock::new_builder()
        .signature(Some(bytes::Bytes::from(vec![0u8; 65])).pack())
        .build();
    Some(witness_lock.as_bytes()).pack()
}

fn get_acp_script(script: Script) -> Option<Script> {
    let mut args = script.args().raw_data()[0..21].to_vec();
    args.extend(vec![1u8]); // omni lock args, 1u8 enables acp
    Some(
        script
            .as_builder()
            .args(args.pack())
            .hash_type(ScriptHashType::Type.into())
            .build(),
    )
}

fn _parse_omni_config(_lock_args: &Bytes) -> Option<OmniLockConfig> {
    // let flag:  = lock_args.raw_data()[0].try_into().ok()?;
    // let hash = H160::from_slice(&lock_args.raw_data()[1..21]).ok()?;
    // let _omni_flag = lock_args.raw_data()[22];
    // let config = OmniLockConfig::new(flag, hash);
    // Some(config)
    todo!()
}
