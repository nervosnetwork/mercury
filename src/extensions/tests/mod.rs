#![allow(dead_code)]

pub mod creation;
mod extension_test;
mod memory_store;

pub use memory_store::MemoryDB;

use crate::extensions::{
    ckb_balance::CkbBalanceExtension, rce_validator::RceValidatorExtension,
    sudt_balance::SUDTBalanceExtension, CKB_EXT_PREFIX, RCE_EXT_PREFIX, SUDT_EXT_PREFIX,
};
use crate::extensions::{BoxedExtension, ExtensionType};
use crate::stores::PrefixStore;
use crate::types::{DeployedScriptConfig, ExtensionsConfig};

use ckb_chain_spec::consensus::Consensus;
use ckb_indexer::{indexer::Indexer, store::Store};
use ckb_sdk::{Address, AddressPayload, NetworkType};
use ckb_types::{bytes::Bytes, packed, prelude::Unpack};
use rand::random;

use std::collections::HashMap;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref GENESIS_OUTPUT_CELL: packed::CellOutput =
        Consensus::default().genesis_block.transaction(0).unwrap().output(0).unwrap();
    pub static ref GENESIS_OUTPUT_ADDRESS: Address =
        Address::new(NetworkType::Testnet, AddressPayload::from(GENESIS_OUTPUT_CELL.lock()));
    pub static ref GENESIS_LOCK_ARGS: Bytes = GENESIS_OUTPUT_CELL.lock().args().unpack();
    pub static ref GENESIS_CAPACITY: u64 = GENESIS_OUTPUT_CELL.capacity().unpack();
}

const EPOCH_INTERVAL: u64 = 10;
const SUDT_CODE_HASH: &str = "c5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4";
const NETWORK_TYPE: NetworkType = NetworkType::Testnet;

enum HashType {
    Data,
    Type,
}

impl Into<packed::Byte> for HashType {
    fn into(self) -> packed::Byte {
        match self {
            HashType::Data => packed::Byte::new(0),
            HashType::Type => packed::Byte::new(1),
        }
    }
}

#[derive(Clone)]
pub struct ExtensionsNeed {
    config: HashMap<String, DeployedScriptConfig>,
    indexer: Arc<Indexer<MemoryDB>>,
    store: PrefixStore<MemoryDB>,
}

pub struct TestHandler {
    inner: HashMap<ExtensionType, ExtensionsNeed>,
}

impl TestHandler {
    pub fn new(config: ExtensionsConfig) -> Self {
        let inner = config
            .enabled_extensions
            .into_iter()
            .map(|(k, v)| {
                let db = MemoryDB::new(k.to_u32().to_string().as_str());
                let indexer = Arc::new(Indexer::new(db.clone(), 100, u64::MAX));

                (
                    k.clone(),
                    ExtensionsNeed {
                        config: v,
                        indexer,
                        store: PrefixStore::new_with_prefix(db, k.to_prefix()),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        TestHandler { inner }
    }

    pub fn ckb_balance_extension(
        &mut self,
    ) -> CkbBalanceExtension<PrefixStore<MemoryDB>, MemoryDB> {
        let db = MemoryDB::new(0u32.to_string().as_str());
        let prefix_store = PrefixStore::new_with_prefix(db.clone(), Bytes::from(*CKB_EXT_PREFIX));
        let indexer = Arc::new(Indexer::new(db, 100, u64::MAX));

        self.inner.insert(
            ExtensionType::CkbBalance,
            ExtensionsNeed {
                config: HashMap::default(),
                indexer: Arc::clone(&indexer),
                store: prefix_store.clone(),
            },
        );

        CkbBalanceExtension::new(prefix_store, indexer, NETWORK_TYPE, HashMap::default())
    }

    // Todo: add `prune_indexer` here
    // fn prune_indexer(&self) {}
}

impl ExtensionsConfig {
    pub fn new() -> Self {
        ExtensionsConfig {
            enabled_extensions: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: ExtensionType, val: HashMap<String, DeployedScriptConfig>) {
        self.enabled_extensions.insert(key, val);
    }

    // pub fn add_ckb_balance_config(mut self) -> Self {
    //     self.insert(ExtensionType::CkbBalance, DeployedScriptConfig::default());
    //     self
    // }
}

pub fn build_extension(
    extension_type: &ExtensionType,
    script_config: HashMap<String, DeployedScriptConfig>,
    indexer: Arc<Indexer<MemoryDB>>,
    store: MemoryDB,
) -> BoxedExtension {
    match extension_type {
        ExtensionType::RceValidator => Box::new(RceValidatorExtension::new(
            PrefixStore::new_with_prefix(store, Bytes::from(*RCE_EXT_PREFIX)),
            script_config,
        )),

        ExtensionType::CkbBalance => Box::new(CkbBalanceExtension::new(
            PrefixStore::new_with_prefix(store, Bytes::from(*CKB_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),

        ExtensionType::SUDTBalance => Box::new(SUDTBalanceExtension::new(
            PrefixStore::new_with_prefix(store, Bytes::from(*SUDT_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),
    }
}

pub fn rand_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
}
