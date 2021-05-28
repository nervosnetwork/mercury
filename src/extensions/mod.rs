pub mod ckb_balance;
pub mod lock_time;
pub mod rce_validator;
pub mod special_cells;
pub mod udt_balance;

#[cfg(test)]
pub mod tests;

use crate::extensions::{
    ckb_balance::CkbBalanceExtension, lock_time::LocktimeExtension,
    rce_validator::RceValidatorExtension, special_cells::SpecialCellsExtension,
    udt_balance::SUDTBalanceExtension,
};
use crate::stores::PrefixStore;
use crate::types::ExtensionsConfig;

use anyhow::Result;
use ckb_indexer::indexer::{DetailedLiveCell, Indexer};
use ckb_indexer::store::Store;
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView, RationalU256};
use ckb_types::prelude::Builder;
use ckb_types::{bytes::Bytes, packed, prelude::Entity};
use parking_lot::RwLock;
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};

use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref RCE_EXT_PREFIX: &'static [u8] = &b"\xFFrce"[..];
    pub static ref CKB_EXT_PREFIX: &'static [u8] = &b"\xFFckb_balance"[..];
    pub static ref UDT_EXT_PREFIX: &'static [u8] = &b"\xFFsudt_balance"[..];
    pub static ref SP_CELL_EXT_PREFIX: &'static [u8] = &b"\xFFspecial_cells"[..];
    pub static ref LOCK_TIME_PREFIX: &'static [u8] = &b"\xFFlock_time"[..];
    pub static ref MATURE_THRESHOLD: RwLock<RationalU256> = RwLock::new(RationalU256::one());
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
    SpecialCells,
    Locktime,
}

impl From<&str> for ExtensionType {
    fn from(s: &str) -> Self {
        match s {
            "ckb_balance" => ExtensionType::CkbBalance,
            "udt_balance" => ExtensionType::UDTBalance,
            "rce_validator" => ExtensionType::RceValidator,
            "special_cells" => ExtensionType::SpecialCells,
            "lock_time" => ExtensionType::Locktime,
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
            ExtensionType::SpecialCells => 48,
            ExtensionType::Locktime => 64,
        }
    }

    fn to_prefix(&self) -> Bytes {
        let prefix = match self {
            ExtensionType::CkbBalance => *CKB_EXT_PREFIX,
            ExtensionType::UDTBalance => *UDT_EXT_PREFIX,
            ExtensionType::RceValidator => *RCE_EXT_PREFIX,
            ExtensionType::SpecialCells => *SP_CELL_EXT_PREFIX,
            ExtensionType::Locktime => *LOCK_TIME_PREFIX,
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

            ExtensionType::SpecialCells => {
                let sp_ext = SpecialCellsExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*SP_CELL_EXT_PREFIX)),
                    Arc::clone(&indexer),
                    net_ty,
                    script_config.clone(),
                );

                results.push(Box::new(sp_ext));
            }

            ExtensionType::Locktime => {
                let locktime_ext = LocktimeExtension::new(
                    PrefixStore::new_with_prefix(store.clone(), Bytes::from(*LOCK_TIME_PREFIX)),
                    Arc::clone(&indexer),
                    net_ty,
                    script_config.clone(),
                );

                results.push(Box::new(locktime_ext));
            }
        }
    }

    Ok(results)
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DetailedCells(pub Vec<DetailedCell>);

impl DetailedCells {
    pub fn push(&mut self, cell: DetailedCell) {
        self.0.push(cell);
    }

    pub fn contains(&self, cell: &DetailedCell) -> bool {
        self.0.contains(cell)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetailedCell {
    pub block_number: BlockNumber,
    #[serde(serialize_with = "encode_mol", deserialize_with = "decode_mol")]
    pub block_hash: packed::Byte32,
    #[serde(serialize_with = "encode_mol", deserialize_with = "decode_mol")]
    pub out_point: packed::OutPoint,
    #[serde(serialize_with = "encode_mol", deserialize_with = "decode_mol")]
    pub cell_output: packed::CellOutput,
    #[serde(serialize_with = "encode_mol", deserialize_with = "decode_mol")]
    pub cell_data: packed::Bytes,
}

impl Hash for DetailedCell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.block_number.hash(state);
        self.block_hash.hash(state);
        self.out_point.hash(state);
        self.cell_output.hash(state);
        self.cell_data.raw_data().hash(state);
    }
}

impl PartialEq for DetailedCell {
    fn eq(&self, other: &DetailedCell) -> bool {
        self.block_number == other.block_number
            && self.block_hash == other.block_hash
            && self.out_point == other.out_point
            && self.cell_output == other.cell_output
            && self.cell_data.raw_data() == other.cell_data.raw_data()
    }
}

impl Eq for DetailedCell {}

impl DetailedCell {
    pub fn from_detailed_live_cell(cell: DetailedLiveCell, out_point: packed::OutPoint) -> Self {
        DetailedCell {
            block_number: cell.block_number,
            block_hash: cell.block_hash,
            cell_output: cell.cell_output,
            cell_data: cell.cell_data,
            out_point,
        }
    }

    pub fn new(
        block_number: BlockNumber,
        block_hash: packed::Byte32,
        cell_output: packed::CellOutput,
        tx_hash: packed::Byte32,
        index: packed::Uint32,
        cell_data: packed::Bytes,
    ) -> Self {
        let out_point = packed::OutPointBuilder::default()
            .tx_hash(tx_hash)
            .index(index)
            .build();

        DetailedCell {
            block_number,
            block_hash,
            out_point,
            cell_output,
            cell_data,
        }
    }
}

fn encode_mol<T: Entity, S: Serializer>(input: &T, s: S) -> Result<S::Ok, S::Error> {
    let bytes = input.as_slice();
    s.serialize_bytes(bytes)
}

fn decode_mol<'de: 'a, 'a, T: Entity, D: Deserializer<'de>>(de: D) -> Result<T, D::Error> {
    let bytes: &'a [u8] = Deserialize::deserialize(de)?;
    let ret = T::from_slice(bytes).map_err(D::Error::custom)?;
    Ok(ret)
}

#[cfg(test)]
mod test {
    use super::*;

    use bincode::{deserialize, serialize};
    use ckb_types::{bytes::Bytes, prelude::*};
    use rand::random;

    fn mock_bytes(len: usize) -> Bytes {
        (0..len).map(|_| random::<u8>()).collect::<Vec<_>>().into()
    }

    fn mock_byte32() -> packed::Byte32 {
        let mut ret = [0u8; 32];
        ret.iter_mut().for_each(|b| *b = random::<u8>());
        ret.pack()
    }

    fn mock_script() -> packed::Script {
        packed::ScriptBuilder::default()
            .args(mock_bytes(32).pack())
            .code_hash(mock_byte32())
            .hash_type(packed::Byte::new(0))
            .build()
    }

    fn mock_cell_output() -> packed::CellOutput {
        packed::CellOutputBuilder::default()
            .type_(Some(mock_script()).pack())
            .lock(mock_script())
            .capacity(random::<u64>().pack())
            .build()
    }

    fn mock_outpoint() -> packed::OutPoint {
        packed::OutPointBuilder::default()
            .tx_hash(mock_byte32())
            .index(random::<u32>().pack())
            .build()
    }

    pub fn mock_detailed_cell() -> DetailedCell {
        DetailedCell {
            block_number: random::<u64>(),
            block_hash: mock_byte32(),
            out_point: mock_outpoint(),
            cell_output: mock_cell_output(),
            cell_data: mock_bytes(16).pack(),
        }
    }

    #[test]
    fn test_detailed_cell_codec() {
        let cell = mock_detailed_cell();

        let bytes = serialize(&cell).unwrap();
        let new = deserialize::<DetailedCell>(&bytes).unwrap();

        assert_eq!(cell, new);
    }

    #[test]
    fn test_detailed_cells_codec() {
        let cells = DetailedCells(vec![
            mock_detailed_cell(),
            mock_detailed_cell(),
            mock_detailed_cell(),
        ]);

        let bytes = serialize(&cells).unwrap();
        let new = deserialize::<DetailedCells>(&bytes).unwrap();

        assert_eq!(cells, new);
    }
}
