mod rce_validator;

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

pub trait Extension {
    fn append(&self, block: &BlockView) -> Result<()>;
    fn rollback(&self, tip_number: BlockNumber, tip_hash: &Byte32) -> Result<()>;
    fn prune(&self, keep_num: u64) -> Result<()>;
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    RceValidator,
}

pub type BoxedExtension = Box<dyn Extension + 'static + Send>;

pub fn build_extensions<S: Store + Clone>(
    config: &ExtensionsConfig,
    store: S,
) -> Result<Vec<BoxedExtension>> {
    let mut results = vec![];
    for (extension_type, script_config) in &config.enabled_extensions {
        match extension_type {
            ExtensionType::RceValidator => {
                let store =
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(&b"\xFFrce"[..]));
                let rce_validator = RceValidatorExtension::new(store, script_config.clone());
                results.push(rce_validator);
            }
        }
    }
    Ok(results)
}
