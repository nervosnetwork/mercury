pub mod anyone_can_pay;
pub mod ckb_balance;
pub mod rce_validator;
pub mod udt_balance;

#[cfg(test)]
pub mod tests;

use crate::extensions::{
    anyone_can_pay::ACPExtension, ckb_balance::CkbBalanceExtension,
    rce_validator::RceValidatorExtension, udt_balance::SUDTBalanceExtension,
};
use crate::stores::PrefixStore;
use crate::types::ExtensionsConfig;

use anyhow::Result;
use ckb_indexer::{indexer::Indexer, store::Store};
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref RCE_EXT_PREFIX: &'static [u8] = &b"\xFFrce"[..];
    pub static ref CKB_EXT_PREFIX: &'static [u8] = &b"\xFFckb_balance"[..];
    pub static ref UDT_EXT_PREFIX: &'static [u8] = &b"\xFFsudt_balance"[..];
    pub static ref ACP_EXT_PREFIX: &'static [u8] = &b"\xFFanyone_can_pay"[..];
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
    UDTBalance,
    RceValidator,
    AnyoneCanPay,
}

impl From<&str> for ExtensionType {
    fn from(s: &str) -> Self {
        match s {
            "ckb_balance" => ExtensionType::CkbBalance,
            "udt_balance" => ExtensionType::UDTBalance,
            "rce_validator" => ExtensionType::RceValidator,
            "anyone_can_pay" => ExtensionType::AnyoneCanPay,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
impl ExtensionType {
    fn to_u32(&self) -> u32 {
        match self {
            ExtensionType::CkbBalance => 0,
            ExtensionType::UDTBalance => 16,
            ExtensionType::RceValidator => 32,
            ExtensionType::AnyoneCanPay => 48,
        }
    }

    fn to_prefix(&self) -> Bytes {
        let prefix = match self {
            ExtensionType::CkbBalance => *CKB_EXT_PREFIX,
            ExtensionType::UDTBalance => *UDT_EXT_PREFIX,
            ExtensionType::RceValidator => *RCE_EXT_PREFIX,
            ExtensionType::AnyoneCanPay => *ACP_EXT_PREFIX,
        };

        Bytes::from(prefix)
    }
}

pub type BoxedExtension = Box<dyn Extension + 'static>;

pub fn build_extensions<S: Store + Clone + 'static, BS: Store + Clone + 'static>(
    net_ty: NetworkType,
    config: &ExtensionsConfig,
    indexer: Arc<Indexer<BS>>,
    store: S,
) -> Result<Vec<BoxedExtension>> {
    let mut results: Vec<BoxedExtension> = Vec::new();

    for (extension_type, script_config) in config.enabled_extensions.iter() {
        match extension_type {
            ExtensionType::RceValidator => {
                let rce_validator = RceValidatorExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*RCE_EXT_PREFIX)),
                    script_config.clone(),
                );
                results.push(Box::new(rce_validator));
            }

            ExtensionType::CkbBalance => {
                let ckb_balance = CkbBalanceExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*CKB_EXT_PREFIX)),
                    Arc::clone(&indexer),
                    net_ty,
                    script_config.clone(),
                );
                results.push(Box::new(ckb_balance));
            }

            ExtensionType::UDTBalance => {
                let sudt_balance = SUDTBalanceExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*UDT_EXT_PREFIX)),
                    Arc::clone(&indexer),
                    net_ty,
                    script_config.clone(),
                );
                results.push(Box::new(sudt_balance));
            }

            ExtensionType::AnyoneCanPay => {
                let acp_ext = ACPExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*ACP_EXT_PREFIX)),
                    Arc::clone(&indexer),
                    net_ty,
                    script_config.clone(),
                );

                results.push(Box::new(acp_ext));
            }
        }
    }

    Ok(results)
}
