mod ckb_balance;
mod rce_validator;
mod sudt_balance;

use crate::{
    extensions::rce_validator::RceValidatorExtension, stores::PrefixStore, types::ExtensionsConfig,
};

use anyhow::Result;
use ckb_indexer::store::Store;
use ckb_types::{
    bytes::Bytes,
    core::{BlockNumber, BlockView},
    packed::Byte32,
};
use serde::{Deserialize, Serialize};

pub trait Extension {
    fn append(&self, block: &BlockView) -> Result<()>;

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &Byte32) -> Result<()>;

    fn prune(&self, tip_number: BlockNumber, tip_hash: &Byte32, keep_num: u64) -> Result<()>;
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    RceValidator,
}

pub type BoxedExtension = Box<dyn Extension + 'static>;

pub fn build_extensions<S: Store + Clone + 'static>(
    config: &ExtensionsConfig,
    store: S,
) -> Result<Vec<BoxedExtension>> {
    let mut results: Vec<BoxedExtension> = vec![];
    for (extension_type, script_config) in &config.enabled_extensions {
        match extension_type {
            ExtensionType::RceValidator => {
                let store =
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(&b"\xFFrce"[..]));
                let rce_validator = RceValidatorExtension::new(store, script_config.clone());
                results.push(Box::new(rce_validator));
            }
        }
    }
    Ok(results)
}

#[derive(Clone, Debug)]
pub struct Array<const LEN: usize>([u8; LEN]);

impl<const LEN: usize> Array<LEN> {
    pub fn from_slice(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), LEN);
        let mut list = [0u8; LEN];
        list.copy_from_slice(slice);

        Array(list)
    }

    pub fn inner(&self) -> [u8; LEN] {
        self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        Vec::from(self.0)
    }
}

pub fn to_fixed_array<const LEN: usize>(input: &[u8]) -> [u8; LEN] {
    assert_eq!(input.len(), LEN);
    let mut list = [0; LEN];
    list.copy_from_slice(input);
    list
}

#[cfg(test)]
mod tests {
    use rand::random;

    use super::*;

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
