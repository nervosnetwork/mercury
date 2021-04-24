#![allow(dead_code)]

mod creation;
mod extension_test;
mod memory_store;

use memory_store::MemoryDB;

use crate::extensions::{
    ckb_balance::CkbBalanceExtension, rce_validator::RceValidatorExtension,
    sudt_balance::SUDTBalanceExtension,
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
        Address::new(NetworkType::Mainnet, AddressPayload::from(GENESIS_OUTPUT_CELL.lock()));
    pub static ref GENESIS_LOCK_ARGS: Bytes = GENESIS_OUTPUT_CELL.lock().args().unpack();
    pub static ref GENESIS_CAPACITY: u64 = GENESIS_OUTPUT_CELL.capacity().unpack();
}

const EPOCH_INTERVAL: u64 = 10;
const SUDT_CODE_HASH: &str = "c5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4a";
const NETWORK_TYPE: NetworkType = NetworkType::Mainnet;

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
    store: MemoryDB,
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
                let indexer = Arc::new(Indexer::new(
                    MemoryDB::new(k.to_u32().to_string().as_str()),
                    100,
                    u64::MAX,
                ));
                let store = MemoryDB::new(k.to_u32().to_string().as_str());

                (
                    k,
                    ExtensionsNeed {
                        config: v,
                        indexer,
                        store,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        TestHandler { inner }
    }

    pub fn extensions_list(&self) -> Vec<BoxedExtension> {
        self.inner
            .iter()
            .map(|(k, v)| {
                build_extension(k, v.config.clone(), Arc::clone(&v.indexer), v.store.clone())
            })
            .collect()
    }

    pub fn ckb_balance_extension(&mut self) -> CkbBalanceExtension<MemoryDB, MemoryDB> {
        let indexer = Arc::new(Indexer::new(
            MemoryDB::new(0u32.to_string().as_str()),
            100,
            u64::MAX,
        ));
        let store = MemoryDB::new(0u32.to_string().as_str());

        self.inner.insert(
            ExtensionType::CkbBalance,
            ExtensionsNeed {
                config: HashMap::default(),
                indexer: Arc::clone(&indexer),
                store: store.clone(),
            },
        );

        CkbBalanceExtension::new(store, indexer, NETWORK_TYPE, HashMap::default())
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

fn build_extension(
    extension_type: &ExtensionType,
    script_config: HashMap<String, DeployedScriptConfig>,
    indexer: Arc<Indexer<MemoryDB>>,
    store: MemoryDB,
) -> BoxedExtension {
    match extension_type {
        ExtensionType::RceValidator => {
            let store = PrefixStore::new_with_prefix(store, Bytes::from(&b"\xFFrce"[..]));
            Box::new(RceValidatorExtension::new(store, script_config))
        }

        ExtensionType::CkbBalance => {
            let store = PrefixStore::new_with_prefix(store, Bytes::from(&b"\xFFckb_balance"[..]));
            Box::new(CkbBalanceExtension::new(
                store,
                Arc::clone(&indexer),
                NETWORK_TYPE,
                script_config,
            ))
        }

        ExtensionType::SUDTBalacne => {
            let store = PrefixStore::new_with_prefix(store, Bytes::from(&b"\xFFsudt_balance"[..]));
            Box::new(SUDTBalanceExtension::new(
                store,
                Arc::clone(&indexer),
                NETWORK_TYPE,
                script_config.clone(),
            ))
        }
    }
}

pub fn rand_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
}
