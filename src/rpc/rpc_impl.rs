use crate::extensions::{
    ckb_balance, rce_validator, sudt_balance, ExtensionType, CKB_EXT_PREFIX, RCE_EXT_PREFIX,
    SUDT_EXT_PREFIX,
};
use crate::rpc::MercuryRpc;
use crate::stores::PrefixStore;

use ckb_indexer::store::Store;
use ckb_types::bytes::Bytes;
use jsonrpc_core::Result as RpcResult;

use std::collections::HashMap;

pub struct MercuryRpcImpl<S> {
    store_map: HashMap<ExtensionType, PrefixStore<S>>,
}

impl<S: Store + Send + Sync + 'static> MercuryRpc for MercuryRpcImpl<S> {
    fn get_ckb_balance(&self, _addr: String) -> RpcResult<u64> {
        Ok(0)
    }

    fn get_sudt_balance(&self, _sudt_id: Bytes, _addr: String) -> RpcResult<u128> {
        Ok(0)
    }
}

impl<S: Store + Clone> MercuryRpcImpl<S> {
    pub fn new(store: S) -> Self {
        let mut store_map = HashMap::new();
        store_map.insert(
            ExtensionType::CkbBalance,
            PrefixStore::new_with_prefix(store.clone(), Bytes::from(*CKB_EXT_PREFIX)),
        );
        store_map.insert(
            ExtensionType::RceValidator,
            PrefixStore::new_with_prefix(store.clone(), Bytes::from(*RCE_EXT_PREFIX)),
        );
        store_map.insert(
            ExtensionType::SUDTBalacne,
            PrefixStore::new_with_prefix(store, Bytes::from(*SUDT_EXT_PREFIX)),
        );

        MercuryRpcImpl { store_map }
    }
}
