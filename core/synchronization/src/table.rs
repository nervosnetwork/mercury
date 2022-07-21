use common::utils::to_fixed_array;
use common::DetailedCell;
use core_storage::relational::BLAKE_160_HSAH_LEN;
use db_xsql::rbatis::{crud_table, Bytes as RbBytes};

use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::{packed, prelude::*, H256};
use serde::{Deserialize, Serialize};

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::hash::{Hash, Hasher};

const HASH256_LEN: usize = 32;

#[macro_export]
macro_rules! single_sql_return {
    ($name: ident, $field: ident, $ty: ident) => {
        #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name {
            pub $field: $ty,
        }
    };
}

single_sql_return!(ScriptHash, script_hash, RbBytes);
single_sql_return!(SyncNumber, block_number, u64);

#[crud_table(table_name: "mercury_consume_info")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConsumeInfoTable {
    pub tx_hash: RbBytes,
    pub output_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: RbBytes,
    pub consumed_tx_hash: RbBytes,
    pub consumed_tx_index: u32,
    pub input_index: u32,
    pub since: RbBytes,
}

impl ConsumeInfoTable {
    pub fn new(
        out_point: packed::OutPoint,
        consumed_block_number: u64,
        consumed_block_hash: RbBytes,
        consumed_tx_hash: RbBytes,
        consumed_tx_index: u32,
        input_index: u32,
        since: u64,
    ) -> Self {
        let tx_hash = to_rb_bytes(&out_point.tx_hash().raw_data());
        let output_index: u32 = out_point.index().unpack();

        ConsumeInfoTable {
            tx_hash,
            output_index,
            consumed_block_number,
            consumed_block_hash,
            consumed_tx_hash,
            consumed_tx_index,
            input_index,
            since: to_rb_bytes(&since.to_be_bytes()),
        }
    }
}

#[crud_table(table_name: "mercury_in_update")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InUpdate {
    pub is_in: bool,
}

#[crud_table(table_name: "mercury_block")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: RbBytes,
    pub block_number: u64,
    pub version: u16,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub parent_hash: RbBytes,
    pub transactions_root: RbBytes,
    pub proposals_hash: RbBytes,
    pub uncles_hash: RbBytes,
    pub uncles: RbBytes,
    pub uncles_count: u32,
    pub dao: RbBytes,
    pub nonce: RbBytes,
    pub proposals: RbBytes,
}

impl From<&BlockView> for BlockTable {
    fn from(block: &BlockView) -> Self {
        let epoch = block.epoch();

        BlockTable {
            block_hash: to_rb_bytes(&block.hash().raw_data()),
            block_number: block.number(),
            version: block.version() as u16,
            compact_target: block.compact_target(),
            block_timestamp: block.timestamp(),
            epoch_number: epoch.number() as u32,
            epoch_index: epoch.index() as u32,
            epoch_length: epoch.length() as u32,
            parent_hash: to_rb_bytes(&block.parent_hash().raw_data()),
            transactions_root: to_rb_bytes(&block.transactions_root().raw_data()),
            proposals_hash: to_rb_bytes(&block.proposals_hash().raw_data()),
            uncles_hash: to_rb_bytes(&block.extra_hash().raw_data()),
            uncles: to_rb_bytes(block.uncles().data().as_slice()),
            uncles_count: block.uncle_hashes().len() as u32,
            dao: to_rb_bytes(&block.dao().raw_data()),
            nonce: to_rb_bytes(&block.nonce().to_be_bytes()),
            proposals: to_rb_bytes(&block.data().proposals().as_bytes()),
        }
    }
}

#[crud_table(table_name: "mercury_transaction")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: RbBytes,
    pub tx_index: u32,
    pub input_count: u32,
    pub output_count: u32,
    pub block_number: u64,
    pub block_hash: RbBytes,
    pub tx_timestamp: u64,
    pub version: u16,
    pub cell_deps: RbBytes,
    pub header_deps: RbBytes,
    pub witnesses: RbBytes,
}

impl TransactionTable {
    pub fn from_view(
        view: &TransactionView,
        id: i64,
        tx_index: u32,
        block_hash: RbBytes,
        block_number: u64,
        tx_timestamp: u64,
    ) -> Self {
        TransactionTable {
            id,
            block_hash,
            block_number,
            tx_index,
            tx_timestamp,
            tx_hash: to_rb_bytes(&view.hash().raw_data()),
            input_count: view.inputs().len() as u32,
            output_count: view.outputs().len() as u32,
            cell_deps: to_rb_bytes(&view.cell_deps().as_bytes()),
            header_deps: to_rb_bytes(&view.header_deps().as_bytes()),
            witnesses: to_rb_bytes(&view.witnesses().as_bytes()),
            version: view.version() as u16,
        }
    }
}

#[crud_table(
    table_name: "mercury_cell" | formats_pg: "
    consumed_block_number:{}::bigint,
    consumed_tx_index:{}::bigint,
    input_index:{}::bigint"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: RbBytes,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: RbBytes,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub capacity: u64,
    pub lock_hash: RbBytes,
    pub lock_code_hash: RbBytes,
    pub lock_args: RbBytes,
    pub lock_script_type: u8,
    pub type_hash: RbBytes,
    pub type_code_hash: RbBytes,
    pub type_args: RbBytes,
    pub type_script_type: u8,
    pub data: RbBytes,
    pub consumed_block_number: Option<u64>,
    pub consumed_block_hash: RbBytes,
    pub consumed_tx_hash: RbBytes,
    pub consumed_tx_index: Option<u32>,
    pub input_index: Option<u32>,
    pub since: RbBytes,
}

impl From<LiveCellTable> for CellTable {
    fn from(s: LiveCellTable) -> Self {
        CellTable {
            id: s.id,
            tx_hash: s.tx_hash,
            output_index: s.output_index,
            block_number: s.block_number,
            block_hash: s.block_hash,
            tx_index: s.tx_index,
            epoch_number: s.epoch_number,
            epoch_index: s.epoch_index,
            epoch_length: s.epoch_length,
            capacity: s.capacity,
            lock_hash: s.lock_hash,
            lock_code_hash: s.lock_code_hash,
            lock_args: s.lock_args,
            lock_script_type: s.lock_script_type,
            type_hash: s.type_hash,
            type_code_hash: s.type_code_hash,
            type_args: s.type_args,
            type_script_type: s.type_script_type,
            data: s.data,
            consumed_block_hash: empty_rb_bytes(),
            consumed_block_number: None,
            consumed_tx_hash: empty_rb_bytes(),
            consumed_tx_index: None,
            input_index: None,
            since: empty_rb_bytes(),
        }
    }
}

impl CellTable {
    pub fn from_cell(
        cell: &packed::CellOutput,
        id: i64,
        tx_hash: RbBytes,
        output_index: u32,
        tx_index: u32,
        block_number: u64,
        block_hash: RbBytes,
        epoch: EpochNumberWithFraction,
        cell_data: &[u8],
    ) -> Self {
        let mut ret = CellTable {
            id,
            tx_hash,
            output_index,
            tx_index,
            block_number,
            block_hash,
            epoch_number: epoch.number() as u32,
            epoch_index: epoch.index() as u32,
            epoch_length: epoch.length() as u32,
            capacity: cell.capacity().unpack(),
            lock_hash: to_rb_bytes(&cell.lock().calc_script_hash().raw_data()),
            lock_code_hash: to_rb_bytes(&cell.lock().code_hash().raw_data()),
            lock_args: to_rb_bytes(&cell.lock().args().raw_data()),
            lock_script_type: cell.lock().hash_type().into(),
            type_hash: to_rb_bytes(&H256::default().0),
            type_code_hash: empty_rb_bytes(),
            type_args: empty_rb_bytes(),
            type_script_type: 0u8,
            data: to_rb_bytes(cell_data),
            consumed_block_number: None,
            consumed_block_hash: empty_rb_bytes(),
            consumed_tx_index: None,
            consumed_tx_hash: empty_rb_bytes(),
            input_index: None,
            since: empty_rb_bytes(),
        };

        if let Some(script) = cell.type_().to_opt() {
            ret.set_type_script_info(&script);
        }

        ret
    }

    pub fn has_type_script(&self) -> bool {
        self.type_hash.inner != H256::default().0.to_vec()
    }

    pub fn set_type_script_info(&mut self, script: &packed::Script) {
        self.type_hash = to_rb_bytes(&script.calc_script_hash().raw_data());
        self.type_code_hash = to_rb_bytes(&script.code_hash().raw_data());
        self.type_args = to_rb_bytes(&script.args().raw_data());
        self.type_script_type = script.hash_type().into();
    }

    pub fn to_lock_script_table(&self) -> ScriptTable {
        ScriptTable {
            script_hash: self.lock_hash.clone(),
            script_args: self.lock_args.clone(),
            script_args_len: self.lock_args.inner.len() as u32,
            script_code_hash: self.lock_code_hash.clone(),
            script_type: self.lock_script_type,
            script_hash_160: to_rb_bytes(self.lock_hash.inner.split_at(BLAKE_160_HSAH_LEN).0),
        }
    }

    pub fn to_type_script_table(&self) -> ScriptTable {
        let type_hash = self.type_hash.clone();
        let type_script_args = self.type_args.clone();

        ScriptTable {
            script_hash: type_hash.clone(),
            script_hash_160: to_rb_bytes(type_hash.inner.split_at(BLAKE_160_HSAH_LEN).0),
            script_args_len: type_script_args.inner.len() as u32,
            script_args: type_script_args,
            script_code_hash: self.type_code_hash.clone(),
            script_type: self.type_script_type,
        }
    }

    fn build_detailed_cell(&self) -> DetailedCell {
        let lock_script = packed::ScriptBuilder::default()
            .code_hash(to_fixed_array::<HASH256_LEN>(&self.lock_code_hash.inner[0..32]).pack())
            .args(self.lock_args.inner.pack())
            .hash_type(packed::Byte::new(self.lock_script_type))
            .build();
        let type_script = if self.type_hash.inner == H256::default().0 {
            None
        } else {
            Some(
                packed::ScriptBuilder::default()
                    .code_hash(
                        H256::from_slice(&self.type_code_hash.inner)
                            .expect("get type code hash")
                            .pack(),
                    )
                    .args(self.type_args.inner.pack())
                    .hash_type(packed::Byte::new(self.type_script_type))
                    .build(),
            )
        };

        let convert_hash = |b: &RbBytes| -> Option<H256> {
            if b.inner.is_empty() {
                None
            } else {
                Some(H256::from_slice(&b.inner).expect("convert hash"))
            }
        };

        let convert_since = |b: &RbBytes| -> Option<u64> {
            if b.inner.is_empty() {
                None
            } else {
                Some(u64::from_be_bytes(to_fixed_array::<8>(&b.inner)))
            }
        };

        DetailedCell {
            epoch_number: EpochNumberWithFraction::new_unchecked(
                self.epoch_number.into(),
                self.epoch_index.into(),
                self.epoch_length.into(),
            )
            .full_value(),
            block_number: self.block_number as u64,
            block_hash: H256::from_slice(&self.block_hash.inner[0..32]).expect("get block hash"),
            tx_index: self.tx_index,
            out_point: packed::OutPointBuilder::default()
                .tx_hash(to_fixed_array::<32>(&self.tx_hash.inner).pack())
                .index((self.output_index as u32).pack())
                .build(),
            cell_output: packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(type_script.pack())
                .capacity(self.capacity.pack())
                .build(),
            cell_data: self.data.inner.clone().into(),
            consumed_block_hash: convert_hash(&self.consumed_block_hash),
            consumed_block_number: self.consumed_block_number,
            consumed_tx_hash: convert_hash(&self.consumed_tx_hash),
            consumed_tx_index: self.consumed_tx_index,
            consumed_input_index: self.input_index,
            since: convert_since(&self.since),
        }
    }
}

impl From<CellTable> for DetailedCell {
    fn from(cell_table: CellTable) -> DetailedCell {
        cell_table.build_detailed_cell()
    }
}

#[crud_table(table_name: "mercury_live_cell")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: RbBytes,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: RbBytes,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub capacity: u64,
    pub lock_hash: RbBytes,
    pub lock_code_hash: RbBytes,
    pub lock_args: RbBytes,
    pub lock_script_type: u8,
    pub type_hash: RbBytes,
    pub type_code_hash: RbBytes,
    pub type_args: RbBytes,
    pub type_script_type: u8,
    pub data: RbBytes,
}

impl From<CellTable> for LiveCellTable {
    fn from(s: CellTable) -> Self {
        LiveCellTable {
            id: s.id,
            tx_hash: s.tx_hash,
            output_index: s.output_index,
            block_number: s.block_number,
            block_hash: s.block_hash,
            tx_index: s.tx_index,
            epoch_number: s.epoch_number,
            epoch_index: s.epoch_index,
            epoch_length: s.epoch_length,
            capacity: s.capacity,
            lock_hash: s.lock_hash,
            lock_code_hash: s.lock_code_hash,
            lock_args: s.lock_args,
            lock_script_type: s.lock_script_type,
            type_hash: s.type_hash,
            type_code_hash: s.type_code_hash,
            type_args: s.type_args,
            type_script_type: s.type_script_type,
            data: s.data,
        }
    }
}

#[crud_table(table_name: "mercury_indexer_cell")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexerCellTable {
    pub id: i64,
    pub block_number: u64,
    pub io_type: u8,
    pub io_index: u32,
    pub tx_hash: RbBytes,
    pub tx_index: u32,
    pub lock_hash: RbBytes,
    pub lock_code_hash: RbBytes,
    pub lock_args: RbBytes,
    pub lock_script_type: u8,
    pub type_hash: RbBytes,
    pub type_code_hash: RbBytes,
    pub type_args: RbBytes,
    pub type_script_type: u8,
}

impl Ord for IndexerCellTable {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.block_number != other.block_number {
            self.block_number.cmp(&other.block_number)
        } else if self.tx_index != other.tx_index {
            self.tx_index.cmp(&other.tx_index)
        } else if self.io_type != other.io_type {
            self.io_type.cmp(&other.io_type)
        } else {
            self.io_index.cmp(&other.io_index)
        }
    }
}

impl PartialOrd for IndexerCellTable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl IndexerCellTable {
    pub fn new_with_empty_scripts(
        block_number: u64,
        io_type: u8,
        io_index: u32,
        tx_hash: RbBytes,
        tx_index: u32,
    ) -> Self {
        IndexerCellTable {
            id: 0,
            block_number,
            io_type,
            io_index,
            tx_hash,
            tx_index,
            lock_hash: empty_rb_bytes(),
            lock_code_hash: empty_rb_bytes(),
            lock_args: empty_rb_bytes(),
            lock_script_type: 0,
            type_hash: empty_rb_bytes(),
            type_code_hash: empty_rb_bytes(),
            type_args: empty_rb_bytes(),
            type_script_type: 0,
        }
    }

    pub fn update_by_cell_table(mut self, cell_table: &CellTable) -> Self {
        self.lock_hash = cell_table.lock_hash.clone();
        self.lock_code_hash = cell_table.lock_code_hash.clone();
        self.lock_args = cell_table.lock_args.clone();
        self.lock_script_type = cell_table.lock_script_type;
        self.type_hash = cell_table.type_hash.clone();
        self.type_code_hash = cell_table.type_code_hash.clone();
        self.type_args = cell_table.type_args.clone();
        self.type_script_type = cell_table.type_script_type;
        self
    }
}

#[crud_table(table_name: "mercury_script")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptTable {
    pub script_hash: RbBytes,
    pub script_hash_160: RbBytes,
    pub script_code_hash: RbBytes,
    pub script_args: RbBytes,
    pub script_type: u8,
    pub script_args_len: u32,
}

#[allow(clippy::from_over_into)]
impl Into<packed::Script> for ScriptTable {
    fn into(self) -> packed::Script {
        packed::ScriptBuilder::default()
            .code_hash(
                H256::from_slice(&self.script_code_hash.inner[0..32])
                    .expect("get code hash h256")
                    .pack(),
            )
            .args(self.script_args.inner.pack())
            .hash_type(packed::Byte::new(self.script_type))
            .build()
    }
}

impl Hash for ScriptTable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.script_hash.inner.hash(state);
    }
}

impl PartialEq for ScriptTable {
    fn eq(&self, other: &Self) -> bool {
        self.script_hash == other.script_hash
            && self.script_code_hash == other.script_code_hash
            && self.script_type == other.script_type
            && self.script_args == other.script_args
    }
}

impl Eq for ScriptTable {}

impl ScriptTable {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut encode = self.script_hash.inner.clone();
        encode.extend_from_slice(&self.script_hash_160.inner);
        encode.extend_from_slice(&self.script_code_hash.inner);
        encode.extend_from_slice(&self.script_args_len.to_be_bytes());
        encode.push(self.script_type);
        encode.extend_from_slice(&self.script_args.inner);
        encode
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        ScriptTable {
            script_hash: to_rb_bytes(&bytes[0..32]),
            script_hash_160: to_rb_bytes(&bytes[32..52]),
            script_code_hash: to_rb_bytes(&bytes[52..84]),
            script_args: to_rb_bytes(&bytes[89..]),
            script_args_len: u32::from_be_bytes(to_fixed_array::<4>(&bytes[84..88])),
            script_type: bytes[88],
        }
    }
}

#[crud_table(table_name: "mercury_sync_status")]
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SyncStatus {
    pub block_number: u64,
}

impl SyncStatus {
    pub fn new(block_number: u64) -> Self {
        SyncStatus { block_number }
    }
}

#[crud_table(table_name: "mercury_canonical_chain")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalChainTable {
    pub block_number: u64,
    pub block_hash: RbBytes,
}

impl Default for CanonicalChainTable {
    fn default() -> Self {
        CanonicalChainTable {
            block_number: 0,
            block_hash: empty_rb_bytes(),
        }
    }
}

impl PartialEq for CanonicalChainTable {
    fn eq(&self, other: &Self) -> bool {
        self.block_number == other.block_number && self.block_hash == other.block_hash
    }
}

impl Eq for CanonicalChainTable {}

impl PartialOrd for CanonicalChainTable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CanonicalChainTable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_number.cmp(&other.block_number)
    }
}

impl CanonicalChainTable {
    pub fn new(block_number: u64, block_hash: RbBytes) -> Self {
        CanonicalChainTable {
            block_number,
            block_hash,
        }
    }
}

#[crud_table(table_name: "mercury_registered_address")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisteredAddressTable {
    pub lock_hash: RbBytes,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexerTxHash {
    pub id: i64,
    pub tx_hash: RbBytes,
}

impl PartialOrd for IndexerTxHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexerTxHash {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

pub fn to_rb_bytes(input: &[u8]) -> RbBytes {
    RbBytes::new(input.to_vec())
}

pub fn empty_rb_bytes() -> RbBytes {
    RbBytes::new(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect()
    }

    fn generate_script_table(args: Vec<u8>) -> ScriptTable {
        ScriptTable {
            script_hash: to_rb_bytes(&rand_bytes(32)),
            script_hash_160: to_rb_bytes(&rand_bytes(20)),
            script_code_hash: to_rb_bytes(&rand_bytes(32)),
            script_args: to_rb_bytes(&args),
            script_args_len: args.len() as u32,
            script_type: 1,
        }
    }

    #[test]
    fn test_script_table_codec() {
        let script = generate_script_table(rand_bytes(20));
        let bytes = script.as_bytes();
        let decode = ScriptTable::from_bytes(&bytes);

        assert_eq!(script, decode);

        let script = generate_script_table(vec![]);
        let bytes = script.as_bytes();
        let decode = ScriptTable::from_bytes(&bytes);

        assert_eq!(script, decode);
    }
}
