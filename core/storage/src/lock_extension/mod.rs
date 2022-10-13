mod omni_lock;

use crate::RelationalStorage;

use common::{NetworkType, Result};
use core_rpc_types::Identity;

use ckb_types::bytes;
use ckb_types::packed::{Bytes, Script, ScriptOpt};
use ckb_types::H256;

use std::future::Future;
use std::pin::Pin;

type QueryTip = fn(
    &'_ RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<u64>> // future API / pollable
            + Send // required by non-single-threaded executors
            + '_,
    >,
>;

type QueryLockScriptsByIdentity = fn(
    Identity,
    &'_ RelationalStorage,
) -> Pin<
    Box<
        dyn Future<Output = Result<Vec<Script>>> // future API / pollable
            + Send // required by non-single-threaded executors
            + '_,
    >,
>;

#[derive(Clone)]
pub struct LockScriptHandler {
    pub name: &'static str,
    pub get_name: fn() -> String,
    pub get_code_hash: fn(network: NetworkType) -> H256,
    pub query_tip: QueryTip, // for test
    pub is_occupied_free:
        fn(lock_args: &Bytes, cell_type: &ScriptOpt, cell_data: &bytes::Bytes) -> bool,
    pub query_lock_scripts_by_identity: QueryLockScriptsByIdentity,
}

impl LockScriptHandler {
    pub fn from_code_hash(
        code_hash: &H256,
        network: NetworkType,
    ) -> Option<&'static LockScriptHandler> {
        inventory::iter::<LockScriptHandler>.into_iter().find(|t| {
            let script_code_hash = (t.get_code_hash)(network);
            &script_code_hash == code_hash
        })
    }

    pub fn from_name<S: AsRef<str>>(test_name: S) -> Option<&'static LockScriptHandler> {
        inventory::iter::<LockScriptHandler>
            .into_iter()
            .find(|t| t.name == test_name.as_ref())
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
