use crate::Storage;
use crate::{lock_extension::LockScriptHandler, RelationalStorage};

use ckb_jsonrpc_types::CellDep;
use common::lazy::{DAO_CODE_HASH, EXTENSION_SCRIPT_INFOS, SUDT_CODE_HASH};
use common::{utils::decode_udt_amount, Result};
use core_rpc_types::Identity;

use ckb_types::bytes;
use ckb_types::core::RationalU256;
use ckb_types::core::ScriptHashType;
use ckb_types::packed::{Bytes, Script, ScriptOpt};
use ckb_types::prelude::*;
use ckb_types::H256;

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
    query_tip,
    is_occupied_free,
    query_lock_scripts_by_identity,
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

dyn_async! {
    async fn query_tip<'a>(
        _self_: &'a LockScriptHandler,
        storage: &'a RelationalStorage,
    ) -> Result<u64> {
        storage.get_tip_number().await
    }
}

dyn_async! {
    async fn query_lock_scripts_by_identity<'a>(
        self_: &'a LockScriptHandler,
        identity: &'a Identity,
        storage: &'a RelationalStorage,
    ) -> Result<Vec<Script>> {
        let mut ret = vec![];
        if let Some(extension_infos) = EXTENSION_SCRIPT_INFOS.get() {
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
