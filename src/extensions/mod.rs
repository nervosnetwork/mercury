pub mod ckb_balance;
pub mod rce_validator;
pub mod sudt_balance;

#[cfg(test)]
mod tests;

use crate::extensions::{
    ckb_balance::CkbBalanceExtension, rce_validator::RceValidatorExtension,
    sudt_balance::SUDTBalanceExtension,
};
use crate::stores::PrefixStore;
use crate::types::ExtensionsConfig;

use anyhow::Result;
use ckb_indexer::{indexer::Indexer, store::Store};
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref RCE_EXT_PREFIX: &'static [u8] = &b"\xFFrce"[..];
    pub static ref CKB_EXT_PREFIX: &'static [u8] = &b"\xFFckb_balance"[..];
    pub static ref SUDT_EXT_PREFIX: &'static [u8] = &b"\xFFsudt_balance"[..];
}

pub trait Extension {
    fn append(&self, block: &BlockView) -> Result<()>;

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &packed::Byte32) -> Result<()>;

    fn prune(
        &self,
        tip_number: BlockNumber,
        tip_hash: &packed::Byte32,
        keep_num: u64,
    ) -> Result<()>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    CkbBalance,
    SUDTBalacne,
    RceValidator,
}

#[cfg(test)]
impl ExtensionType {
    fn to_u32(&self) -> u32 {
        match self {
            ExtensionType::CkbBalance => 0,
            ExtensionType::SUDTBalacne => 16,
            ExtensionType::RceValidator => 32,
        }
    }
}

pub type BoxedExtension = Box<dyn Extension + 'static>;

pub fn build_extensions<S: Store + Clone + 'static, BS: Store + Clone + 'static>(
    config: &ExtensionsConfig,
    indexer: Arc<Indexer<BS>>,
    store: S,
) -> Result<Vec<BoxedExtension>> {
    let mut results: Vec<BoxedExtension> = Vec::new();

    for (extension_type, script_config) in config.enabled_extensions.iter() {
        match extension_type {
            ExtensionType::RceValidator => {
                let store =
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*RCE_EXT_PREFIX));
                let rce_validator = RceValidatorExtension::new(store, script_config.clone());
                results.push(Box::new(rce_validator));
            }

            ExtensionType::CkbBalance => {
                let store =
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*CKB_EXT_PREFIX));
                let ckb_balance =
                    CkbBalanceExtension::new(store, Arc::clone(&indexer), script_config.clone());
                results.push(Box::new(ckb_balance));
            }

            ExtensionType::SUDTBalacne => {
                let store =
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*SUDT_EXT_PREFIX));
                let sudt_balance =
                    SUDTBalanceExtension::new(store, Arc::clone(&indexer), script_config.clone());
                results.push(Box::new(sudt_balance));
            }
        }
    }

    Ok(results)
}

pub fn to_fixed_array<const LEN: usize>(input: &[u8]) -> [u8; LEN] {
    assert_eq!(input.len(), LEN);
    let mut list = [0; LEN];
    list.copy_from_slice(input);
    list
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
    }

    #[test]
    fn test_to_fixed_array() {
        let bytes = rand_bytes(3);
        let a = to_fixed_array::<3>(&bytes);
        let mut b = [0u8; 3];
        b.copy_from_slice(&bytes);

        assert_eq!(a, b);
    }
}
