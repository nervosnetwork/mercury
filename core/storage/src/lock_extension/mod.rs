mod omni_lock;

use crate::RelationalStorage;

use common::utils::ScriptInfo;
use common::{NetworkType, Result};
use core_rpc_types::Identity;

use ckb_types::core::RationalU256;
use ckb_types::packed::{Bytes, Script, ScriptOpt};
use ckb_types::H256;

use std::future::Future;
use std::pin::Pin;

pub trait LockScript {
    // fn get_lock_info(&self) -> ScriptInfo;

    fn query_lock_scripts_by_identity(
        &self,
        identity: &Identity,
        storage: &RelationalStorage,
    ) -> Result<Vec<Script>>;

    // fn caculate_occupied(&self, lock_args: &Bytes, type_: &ScriptOpt, data: &Bytes) -> u64;

    // fn is_unlock(&self, from: RationalU256, end: Option<RationalU256>) -> bool;

    // fn is_anyone_can_pay(&self, lock_args: Option<Bytes>) -> bool;

    // fn address_to_identity(&self, address: &str) -> Result<Identity>;
}

#[derive(Clone)]
pub struct LockScriptHandler {
    pub name: &'static str,
    pub get_name: fn() -> String,
    pub get_code_hash: fn(network: NetworkType) -> H256,
    pub query_lock_scripts_by_identity: fn(
        &'_ RelationalStorage,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<u64>> // future API / pollable
                + Send // required by non-single-threaded executors
                + '_, // may capture `req`, which is only valid for the `'_` lifetime
        >,
    >,
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
