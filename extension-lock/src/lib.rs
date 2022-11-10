pub mod omni_lock;

use common::{lazy::EXTENSION_LOCK_SCRIPT_INFOS, lazy::EXTENSION_LOCK_SCRIPT_NAMES, Result};
use core_rpc_types::{ExtraFilter, Identity, LockFilter};
use core_storage::RelationalStorage;

use ckb_types::bytes;
use ckb_types::packed::{Bytes, CellOutput, Script, ScriptOpt};
use ckb_types::prelude::Unpack;
use ckb_types::H256;

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

type QueryLockScriptsByIdentity = for<'a> fn(
    &'a H256,
    &'a Identity,
    &'a LockFilter,
    &'a RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<Vec<Script>>> // future API / pollable
            + Send // required by non-single-threaded executors
            + 'a,
    >,
>;

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

#[derive(Clone)]
pub struct LockScriptHandler {
    pub name: &'static str,
    pub is_occupied_free:
        fn(lock_args: &Bytes, cell_type: &ScriptOpt, cell_data: &bytes::Bytes) -> bool,
    pub query_lock_scripts_by_identity: QueryLockScriptsByIdentity,
    pub filter_script: fn(script: &Script, lock_filter: &LockFilter) -> bool,
    pub generate_extra_filter: fn(type_script: Script) -> Option<ExtraFilter>,
    pub script_to_identity: fn(&Script) -> Option<Identity>,
    pub can_be_pooled_ckb: fn() -> bool,
    pub can_be_pooled_udt: fn() -> bool,
    pub caculate_output_current_and_extra_capacity:
        fn(cell: &CellOutput, cell_data: &Bytes) -> Option<(u64, u64)>,
    pub get_witness_lock_placeholder: fn(script: &Script) -> Option<bytes::Bytes>,
    pub insert_script_deps: fn(lock_name: &str, script_deps: &mut BTreeSet<String>),
    pub get_acp_script: fn(script: Script) -> Option<Script>,
    pub get_normal_script: fn(script: Script) -> Option<Script>,
    pub is_mode_supported: fn(lock_args: &Bytes) -> bool,
    pub is_anyone_can_pay: fn(lock_args: &Bytes) -> bool,
}

impl LockScriptHandler {
    fn from_code_hash(code_hash: &H256) -> Option<&'static LockScriptHandler> {
        let extension_script_names = EXTENSION_LOCK_SCRIPT_NAMES.get()?;
        let script = extension_script_names.get(code_hash)?;
        LockScriptHandler::from_name(script)
    }

    fn from_name<S: AsRef<str>>(name: S) -> Option<&'static LockScriptHandler> {
        inventory::iter::<LockScriptHandler>
            .into_iter()
            .find(|t| t.name == name.as_ref())
    }

    pub fn get_script_name(code_hash: &H256) -> Option<&String> {
        let extension_script_names = EXTENSION_LOCK_SCRIPT_NAMES.get()?;
        extension_script_names.get(code_hash)
    }

    pub async fn query_lock_scripts_by_identity(
        ident: &Identity,
        storage: &RelationalStorage,
        lock_filters: &HashMap<&H256, LockFilter>,
    ) -> Result<Vec<Script>> {
        let mut ret = vec![];
        for lock_handler in inventory::iter::<LockScriptHandler>.into_iter() {
            if let Some(extension_infos) = EXTENSION_LOCK_SCRIPT_INFOS.get() {
                if let Some(info) = extension_infos.get(lock_handler.name) {
                    let code_hash = info.script.code_hash().unpack();
                    let lock_filter = lock_filters.get(&code_hash);
                    if !lock_filters.is_empty() && lock_filter.is_none() {
                        continue;
                    }
                    let lock_filter = lock_filter.map(ToOwned::to_owned).unwrap_or_default();
                    let mut scripts = (lock_handler.query_lock_scripts_by_identity)(
                        &code_hash,
                        ident,
                        &lock_filter,
                        storage,
                    )
                    .await?;
                    ret.append(&mut scripts)
                }
            }
        }
        Ok(ret)
    }

    pub fn script_to_identity(script: &Script) -> Option<Identity> {
        let code_hash = script.code_hash().unpack();
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&code_hash) {
            return (lock_handler.script_to_identity)(script);
        }
        None
    }

    pub fn get_normal_script(script: Script) -> Option<Script> {
        let code_hash = script.code_hash().unpack();
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&code_hash) {
            return (lock_handler.get_normal_script)(script);
        }
        None
    }

    pub fn get_acp_script(script: Script) -> Option<Script> {
        let code_hash = script.code_hash().unpack();
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&code_hash) {
            return (lock_handler.get_acp_script)(script);
        }
        None
    }

    pub fn insert_script_deps(code_hash: &H256, script_deps: &mut BTreeSet<String>) {
        if let Some(handler) = LockScriptHandler::from_code_hash(code_hash) {
            (handler.insert_script_deps)(handler.name, script_deps);
        }
    }

    pub fn get_can_be_pooled_ckb_lock(
        lock_filters: &mut HashMap<&H256, LockFilter>,
        lock_filter: LockFilter,
    ) {
        if let Some(extension_script_infos) = EXTENSION_LOCK_SCRIPT_NAMES.get() {
            for code_hash in extension_script_infos.keys() {
                if let Some(lock_handler) = LockScriptHandler::from_code_hash(code_hash) {
                    if (lock_handler.can_be_pooled_ckb)() {
                        lock_filters.insert(code_hash, lock_filter);
                    }
                };
            }
        }
    }

    pub fn get_can_be_pooled_udt_lock(
        lock_filters: &mut HashMap<&H256, LockFilter>,
        lock_filter: LockFilter,
    ) {
        if let Some(extension_script_infos) = EXTENSION_LOCK_SCRIPT_NAMES.get() {
            for code_hash in extension_script_infos.keys() {
                if let Some(lock_handler) = LockScriptHandler::from_code_hash(code_hash) {
                    if (lock_handler.can_be_pooled_udt)() {
                        lock_filters.insert(code_hash, lock_filter);
                    }
                };
            }
        }
    }

    pub fn caculate_output_current_and_extra_capacity(
        cell: &CellOutput,
        cell_data: &Bytes,
    ) -> Option<(u64, u64)> {
        let code_hash = cell.lock().code_hash().unpack();
        let handler = LockScriptHandler::from_code_hash(&code_hash)?;
        (handler.caculate_output_current_and_extra_capacity)(cell, cell_data)
    }

    pub fn is_occupied_free(cell_output: &CellOutput, cell_data: &bytes::Bytes) -> bool {
        let code_hash = cell_output.lock().code_hash().unpack();
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&code_hash) {
            (lock_handler.is_occupied_free)(
                &cell_output.lock().args(),
                &cell_output.type_(),
                cell_data,
            )
        } else {
            false
        }
    }

    pub fn get_witness_lock_placeholder(script: &Script) -> Option<bytes::Bytes> {
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&script.code_hash().unpack())
        {
            (lock_handler.get_witness_lock_placeholder)(script)
        } else {
            None
        }
    }

    pub fn filter_script(script: Script, lock_filter: &LockFilter) -> Option<Script> {
        let code_hash = &script.code_hash().unpack();
        if let Some(handler) = LockScriptHandler::from_code_hash(code_hash) {
            Some(script).filter(|script| (handler.filter_script)(script, lock_filter))
        } else {
            None
        }
    }

    pub fn generate_extra_filter(
        lock_code_hash: &H256,
        type_script: Script,
    ) -> Option<ExtraFilter> {
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(lock_code_hash) {
            (lock_handler.generate_extra_filter)(type_script)
        } else {
            Some(ExtraFilter::Frozen)
        }
    }
}

impl std::hash::Hash for LockScriptHandler {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.name.as_bytes());
        state.finish();
    }
}

impl std::fmt::Debug for LockScriptHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "lock name: {}", self.name)
    }
}

impl PartialEq for LockScriptHandler {
    fn eq(&self, other: &LockScriptHandler) -> bool {
        self.name == other.name
    }
}

impl Eq for LockScriptHandler {}

inventory::collect!(LockScriptHandler);
