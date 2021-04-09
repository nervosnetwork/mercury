use crate::types::ExtensionsConfig;
use anyhow::Result;
use ckb_types::core::BlockView;

pub trait Extension {
    fn append(&self, block: &BlockView) -> Result<()>;
    fn rollback(&self) -> Result<()>;
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    RceValidator,
}

pub type BoxedExtension = Box<dyn Extension + 'static + Send>;

pub fn build_extensions(config: &ExtensionsConfig) -> Result<Vec<BoxedExtension>> {
    unimplemented!()
}
