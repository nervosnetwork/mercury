use crate::extensions::{
    ckb_balance, sudt_balance, ExtensionType, CKB_EXT_PREFIX, RCE_EXT_PREFIX, SUDT_EXT_PREFIX,
};
use crate::rpc::MercuryRpc;
use crate::stores::PrefixStore;

use crate::utils::{parse_address, to_fixed_array};

use ckb_indexer::store::Store;
use ckb_types::bytes::Bytes;
use jsonrpc_core::{Error, Result as RpcResult};

use std::collections::HashMap;

pub struct MercuryRpcImpl<S> {
    store_map: HashMap<ExtensionType, PrefixStore<S>>,
}

impl<S: Store + Send + Sync + 'static> MercuryRpc for MercuryRpcImpl<S> {
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>> {
        let address = parse_address(&addr).map_err(|e| Error::invalid_params(e.to_string()))?;
        let key: Vec<u8> = ckb_balance::Key::CkbAddress(&address.to_string()).into();

        self.store_map
            .get(&ExtensionType::CkbBalance)
            .unwrap()
            .get(&key)
            .map_err(|_| Error::internal_error())?
            .map_or_else(
                || Ok(None),
                |bytes| Ok(Some(u64::from_be_bytes(to_fixed_array(&bytes)))),
            )
    }

    fn get_sudt_balance(&self, sudt_hash: Bytes, addr: String) -> RpcResult<Option<u128>> {
        let address = parse_address(&addr).map_err(|e| Error::invalid_params(e.to_string()))?;
        let mut encoded = sudt_hash.to_vec();
        encoded.extend_from_slice(&address.to_string().as_bytes());
        let key: Vec<u8> = sudt_balance::Key::Address(&encoded).into();

        self.store_map
            .get(&ExtensionType::SUDTBalance)
            .unwrap()
            .get(&key)
            .map_err(|_| Error::internal_error())?
            .map_or_else(
                || Ok(None),
                |bytes| Ok(Some(u128::from_be_bytes(to_fixed_array(&bytes)))),
            )
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
            ExtensionType::SUDTBalance,
            PrefixStore::new_with_prefix(store, Bytes::from(*SUDT_EXT_PREFIX)),
        );

        MercuryRpcImpl { store_map }
    }
}
