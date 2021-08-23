#![allow(clippy::mutable_key_type, clippy::from_over_into)]
pub mod rce_validator;

use common::anyhow::Result;

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::packed;

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

// #[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
// #[serde(rename_all = "snake_case")]
// pub enum ExtensionType {
//     ScriptHash,
//     CkbBalance,
//     UDTBalance,
//     RceValidator,
//     SpecialCells,
//     Locktime,
// }

// impl From<&str> for ExtensionType {
//     fn from(s: &str) -> Self {
//         match s {
//             "script_hash" => ExtensionType::ScriptHash,
//             "ckb_balance" => ExtensionType::CkbBalance,
//             "udt_balance" => ExtensionType::UDTBalance,
//             "rce_validator" => ExtensionType::RceValidator,
//             "special_cells" => ExtensionType::SpecialCells,
//             "lock_time" => ExtensionType::Locktime,
//             _ => unreachable!(),
//         }
//     }
// }

// #[cfg(test)]
// impl ExtensionType {
//     fn to_u32(&self) -> u32 {
//         match self {
//             ExtensionType::CkbBalance => 0,
//             ExtensionType::UDTBalance => 16,
//             ExtensionType::RceValidator => 32,
//             ExtensionType::SpecialCells => 48,
//             ExtensionType::Locktime => 64,
//             ExtensionType::ScriptHash => 80,
//         }
//     }

//     fn to_prefix(&self) -> Bytes {
//         let prefix = match self {
//             ExtensionType::CkbBalance => *CKB_EXT_PREFIX,
//             ExtensionType::UDTBalance => *UDT_EXT_PREFIX,
//             ExtensionType::RceValidator => *RCE_EXT_PREFIX,
//             ExtensionType::SpecialCells => *SP_CELL_EXT_PREFIX,
//             ExtensionType::Locktime => *LOCK_TIME_PREFIX,
//             ExtensionType::ScriptHash => *SCRIPT_HASH_EXT_PREFIX,
//         };

//         Bytes::from(prefix)
//     }
// }

// pub type BoxedExtension = Box<dyn Extension + 'static>;

// pub fn build_extensions<S: Store + Clone + 'static, BS: Store + Clone + 'static>(
//     net_ty: NetworkType,
//     config: &ExtensionsConfig,
//     indexer: Arc<Indexer<BS>>,
//     store: S,
// ) -> Result<Vec<BoxedExtension>> {
//     let mut results: Vec<BoxedExtension> = Vec::new();

//     for (extension_type, script_config) in config.enabled_extensions.iter() {
//         match extension_type {
//             ExtensionType::RceValidator => {
//                 let rce_validator = RceValidatorExtension::new(
//                     PrefixStore::new_with_prefix(store.clone(), Bytes::from(*RCE_EXT_PREFIX)),
//                     script_config.clone(),
//                 );
//                 results.push(Box::new(rce_validator));
//             }

//             ExtensionType::CkbBalance => {
//                 let ckb_balance = CkbBalanceExtension::new(
//                     PrefixStore::new_with_prefix(store.clone(), Bytes::from(*CKB_EXT_PREFIX)),
//                     Arc::clone(&indexer),
//                     net_ty,
//                     script_config.clone(),
//                 );
//                 results.push(Box::new(ckb_balance));
//             }

//             ExtensionType::UDTBalance => {
//                 let sudt_balance = UDTBalanceExtension::new(
//                     PrefixStore::new_with_prefix(store.clone(), Bytes::from(*UDT_EXT_PREFIX)),
//                     Arc::clone(&indexer),
//                     net_ty,
//                     script_config.clone(),
//                 );
//                 results.push(Box::new(sudt_balance));
//             }

//             ExtensionType::SpecialCells => {
//                 let sp_ext = SpecialCellsExtension::new(
//                     PrefixStore::new_with_prefix(store.clone(), Bytes::from(*SP_CELL_EXT_PREFIX)),
//                     Arc::clone(&indexer),
//                     net_ty,
//                     script_config.clone(),
//                 );

//                 results.push(Box::new(sp_ext));
//             }

//             ExtensionType::Locktime => {
//                 let locktime_ext = LocktimeExtension::new(
//                     PrefixStore::new_with_prefix(store.clone(), Bytes::from(*LOCK_TIME_PREFIX)),
//                     Arc::clone(&indexer),
//                     net_ty,
//                     script_config.clone(),
//                 );

//                 results.push(Box::new(locktime_ext));
//             }

//             ExtensionType::ScriptHash => {
//                 let script_hash_ext = ScriptHashExtension::new(
//                     PrefixStore::new_with_prefix(
//                         store.clone(),
//                         Bytes::from(*SCRIPT_HASH_EXT_PREFIX),
//                     ),
//                     Arc::clone(&indexer),
//                     net_ty,
//                     script_config.clone(),
//                 );

//                 results.push(Box::new(script_hash_ext));
//             }
//         }
//     }

//     Ok(results)
// }
