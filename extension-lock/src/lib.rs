pub mod omni_lock;

use common::{
    lazy::{EXTENSION_LOCK_SCRIPT_INFOS, EXTENSION_LOCK_SCRIPT_NAMES},
    Result,
};
use core_rpc_types::{ExtraFilter, Identity, ScriptGroup};
use core_storage::RelationalStorage;

use ckb_types::bytes;
use ckb_types::packed::{Bytes, BytesOpt, Script, ScriptOpt};
use ckb_types::prelude::Unpack;
use ckb_types::H256;

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

type QueryLockScriptsByIdentity = for<'a> fn(
    &'a H256,
    &'a Identity,
    &'a RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<Vec<Script>>> // future API / pollable
            + Send // required by non-single-threaded executors
            + 'a,
    >,
>;

#[derive(Clone)]
pub struct LockScriptHandler {
    pub name: &'static str,
    pub is_occupied_free:
        fn(lock_args: &Bytes, cell_type: &ScriptOpt, cell_data: &bytes::Bytes) -> bool,
    pub query_lock_scripts_by_identity: QueryLockScriptsByIdentity,
    pub generate_extra_filter: fn(type_script: Script) -> Option<ExtraFilter>,
    pub script_to_identity: fn(&Script) -> Option<Identity>,
    pub can_be_pooled_ckb: fn() -> bool,
    pub get_witness_lock_placeholder: fn(script_group: &ScriptGroup) -> BytesOpt,
    pub insert_script_deps: fn(lock_name: &str, script_deps: &mut BTreeSet<String>),
    pub get_acp_script: fn(script: Script) -> Option<Script>,
}

impl LockScriptHandler {
    pub fn from_code_hash(code_hash: &H256) -> Option<&'static LockScriptHandler> {
        let extension_script_names = EXTENSION_LOCK_SCRIPT_NAMES.get()?;
        let script = extension_script_names.get(code_hash)?;
        LockScriptHandler::from_name(script)
    }

    pub fn from_name<S: AsRef<str>>(name: S) -> Option<&'static LockScriptHandler> {
        inventory::iter::<LockScriptHandler>
            .into_iter()
            .find(|t| t.name == name.as_ref())
    }

    pub async fn query_lock_scripts_by_identity(
        ident: &Identity,
        storage: &RelationalStorage,
    ) -> Result<Vec<Script>> {
        let mut ret = vec![];
        for lock_handler in inventory::iter::<LockScriptHandler>.into_iter() {
            if let Some(extension_infos) = EXTENSION_LOCK_SCRIPT_INFOS.get() {
                if let Some(info) = extension_infos.get(lock_handler.name) {
                    let code_hash = info.script.code_hash().unpack();
                    let mut scripts =
                        (lock_handler.query_lock_scripts_by_identity)(&code_hash, ident, storage)
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

    pub fn get_acp_script(script: Script) -> Option<Script> {
        let code_hash = script.code_hash().unpack();
        if let Some(lock_handler) = LockScriptHandler::from_code_hash(&code_hash) {
            return (lock_handler.get_acp_script)(script);
        }
        None
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
