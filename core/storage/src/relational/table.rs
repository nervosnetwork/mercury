use crate::relational::{empty_bson_bytes, to_bson_bytes};

use common::utils::to_fixed_array;
use db_xsql::rbatis::crud_table;

use bson::Binary;
use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::{packed, prelude::*, H256};

use serde::{Deserialize, Serialize};

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::hash::{Hash, Hasher};

pub type BsonBytes = Binary;

const BLAKE_160_HSAH_LEN: usize = 20;

#[crud_table(
    table_name: "mercury_block" | formats_pg: "
    block_hash:{}::bytea,
    parent_hash:{}::bytea,
    transactions_root:{}::bytea,
    proposals_hash:{}::bytea,
    uncles_hash:{}::bytea,
    dao:{}::bytea,
    nonce:{}::bytea,
    proposals:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockTable {
    pub block_hash: BsonBytes,
    pub block_number: u64,
    pub version: u16,
    pub compact_target: u32,
    pub block_timestamp: u64,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub parent_hash: BsonBytes,
    pub transactions_root: BsonBytes,
    pub proposals_hash: BsonBytes,
    pub uncles_hash: BsonBytes,
    pub dao: BsonBytes,
    pub nonce: BsonBytes,
    pub proposals: BsonBytes,
}

impl From<&BlockView> for BlockTable {
    fn from(block: &BlockView) -> Self {
        let epoch = block.epoch();

        BlockTable {
            block_hash: to_bson_bytes(&block.hash().raw_data()),
            block_number: block.number(),
            version: block.version() as u16,
            compact_target: block.compact_target(),
            block_timestamp: block.timestamp(),
            epoch_number: epoch.number() as u32,
            epoch_index: epoch.index() as u32,
            epoch_length: epoch.length() as u32,
            parent_hash: to_bson_bytes(&block.parent_hash().raw_data()),
            transactions_root: to_bson_bytes(&block.transactions_root().raw_data()),
            proposals_hash: to_bson_bytes(&block.proposals_hash().raw_data()),
            uncles_hash: to_bson_bytes(&block.uncles_hash().raw_data()),
            dao: to_bson_bytes(&block.dao().raw_data()),
            nonce: to_bson_bytes(&block.nonce().to_be_bytes()),
            proposals: to_bson_bytes(&block.data().proposals().as_bytes()),
        }
    }
}

#[crud_table(
    table_name: "mercury_transaction" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    cell_deps:{}::bytea,
    header_deps:{}::bytea,
    witnesses:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub tx_index: u32,
    pub input_count: u32,
    pub output_count: u32,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub tx_timestamp: u64,
    pub version: u16,
    pub cell_deps: BsonBytes,
    pub header_deps: BsonBytes,
    pub witnesses: BsonBytes,
}

impl TransactionTable {
    pub fn from_view(
        view: &TransactionView,
        id: i64,
        tx_index: u32,
        block_hash: BsonBytes,
        block_number: u64,
        tx_timestamp: u64,
    ) -> Self {
        TransactionTable {
            id,
            block_hash,
            block_number,
            tx_index,
            tx_timestamp,
            tx_hash: to_bson_bytes(&view.hash().raw_data()),
            input_count: view.inputs().len() as u32,
            output_count: view.outputs().len() as u32,
            cell_deps: to_bson_bytes(&view.cell_deps().as_bytes()),
            header_deps: to_bson_bytes(&view.header_deps().as_bytes()),
            witnesses: to_bson_bytes(&view.witnesses().as_bytes()),
            version: view.version() as u16,
        }
    }
}

#[crud_table(
    table_name: "mercury_cell" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    lock_hash:{}::bytea,
    lock_code_hash:{}::bytea,
    lock_args:{}::bytea,
    type_hash:{}::bytea,
    type_code_hash:{}::bytea,
    type_args:{}::bytea,
    type_script_type:{}::smallint,
    data:{}::bytea,
    consumed_block_hash:{}::bytea,
    consumed_tx_hash:{}::bytea,
    since:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub capacity: u64,
    pub lock_hash: BsonBytes,
    pub lock_code_hash: BsonBytes,
    pub lock_args: BsonBytes,
    pub lock_script_type: u8,
    pub type_hash: BsonBytes,
    pub type_code_hash: BsonBytes,
    pub type_args: BsonBytes,
    pub type_script_type: u8,
    pub data: BsonBytes,
}

impl Default for CellTable {
    fn default() -> Self {
        CellTable {
            tx_hash: empty_bson_bytes(),
            block_hash: empty_bson_bytes(),
            lock_hash: empty_bson_bytes(),
            lock_code_hash: empty_bson_bytes(),
            lock_args: empty_bson_bytes(),
            type_hash: empty_bson_bytes(),
            type_code_hash: empty_bson_bytes(),
            type_args: empty_bson_bytes(),
            data: empty_bson_bytes(),
            ..Default::default()
        }
    }
}

impl CellTable {
    pub fn from_cell(
        cell: &packed::CellOutput,
        id: i64,
        tx_hash: BsonBytes,
        output_index: u32,
        tx_index: u32,
        block_number: u64,
        block_hash: BsonBytes,
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
            lock_hash: to_bson_bytes(&cell.lock().calc_script_hash().raw_data()),
            lock_code_hash: to_bson_bytes(&cell.lock().code_hash().raw_data()),
            lock_args: to_bson_bytes(&cell.lock().args().raw_data()),
            lock_script_type: cell.lock().hash_type().into(),
            type_hash: to_bson_bytes(&H256::default().0),
            type_code_hash: empty_bson_bytes(),
            type_args: empty_bson_bytes(),
            type_script_type: 0u8,
            data: to_bson_bytes(&cell_data),
        };

        if let Some(script) = cell.type_().to_opt() {
            ret.set_type_script_info(&script);
        }

        ret
    }

    pub fn has_type_script(&self) -> bool {
        self.type_hash.bytes != H256::default().0.to_vec()
    }

    pub fn set_type_script_info(&mut self, script: &packed::Script) {
        self.type_hash = to_bson_bytes(&script.calc_script_hash().raw_data());
        self.type_code_hash = to_bson_bytes(&script.code_hash().raw_data());
        self.type_args = to_bson_bytes(&script.args().raw_data());
        self.type_script_type = script.hash_type().into();
    }

    pub fn to_lock_script_table(&self) -> ScriptTable {
        ScriptTable {
            script_hash: self.lock_hash.clone(),
            script_args: self.lock_args.clone(),
            script_args_len: self.lock_args.bytes.len() as u32,
            script_code_hash: self.lock_code_hash.clone(),
            script_type: self.lock_script_type,
            script_hash_160: to_bson_bytes(self.lock_hash.bytes.split_at(BLAKE_160_HSAH_LEN).0),
        }
    }

    pub fn to_type_script_table(&self) -> ScriptTable {
        let type_hash = self.type_hash.clone();
        let type_script_args = self.type_args.clone();

        ScriptTable {
            script_hash: type_hash.clone(),
            script_hash_160: to_bson_bytes(&type_hash.bytes.split_at(BLAKE_160_HSAH_LEN).0),
            script_args_len: type_script_args.bytes.len() as u32,
            script_args: type_script_args,
            script_code_hash: self.type_code_hash.clone(),
            script_type: self.type_script_type,
        }
    }
}

#[crud_table(
    table_name: "mercury_consume_info" | formats_pg: "
    tx_hash:{}::bytea,
    consumed_block_hash:{}::bytea,
    consumed_tx_hash:{}::bytea,
    since:{}::bytea"
)]
pub struct ConsumeInfoTable {
    pub tx_hash: BsonBytes,
    pub output_index: u32,
    pub consumed_block_number: u64,
    pub consumed_block_hash: BsonBytes,
    pub consumed_tx_hash: BsonBytes,
    pub consumed_tx_index: u32,
    pub input_index: u32,
    pub since: BsonBytes,
}

impl ConsumeInfoTable {
    pub fn new(
        out_point: packed::OutPoint,
        consumed_block_number: u64,
        consumed_block_hash: BsonBytes,
        consumed_tx_hash: BsonBytes,
        consumed_tx_index: u32,
        input_index: u32,
        since: u64,
    ) -> Self {
        let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
        let output_index: u32 = out_point.index().unpack();

        ConsumeInfoTable {
            tx_hash,
            output_index,
            consumed_block_number,
            consumed_block_hash,
            consumed_tx_hash,
            consumed_tx_index,
            input_index,
            since: to_bson_bytes(&since.to_be_bytes()),
        }
    }
}

#[crud_table(
    table_name: "mercury_live_cell" | formats_pg: "
    tx_hash:{}::bytea,
    block_hash:{}::bytea,
    lock_hash:{}::bytea,
    lock_code_hash:{}::bytea,
    lock_args:{}::bytea,
    type_hash:{}::bytea,
    type_code_hash:{}::bytea,
    type_args:{}::bytea,
    type_script_type:{}::int,
    data:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveCellTable {
    pub id: i64,
    pub tx_hash: BsonBytes,
    pub output_index: u32,
    pub tx_index: u32,
    pub block_number: u64,
    pub block_hash: BsonBytes,
    pub epoch_number: u32,
    pub epoch_index: u32,
    pub epoch_length: u32,
    pub capacity: u64,
    pub lock_hash: BsonBytes,
    pub lock_code_hash: BsonBytes,
    pub lock_args: BsonBytes,
    pub lock_script_type: u8,
    pub type_hash: BsonBytes,
    pub type_code_hash: BsonBytes,
    pub type_args: BsonBytes,
    pub type_script_type: u8,
    pub data: BsonBytes,
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

#[crud_table(
    table_name: "mercury_script" | formats_pg:"
    script_hash:{}::bytea,
    script_hash_160:{}::bytea,
    script_code_hash:{}::bytea,
    script_args:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptTable {
    pub script_hash: BsonBytes,
    pub script_hash_160: BsonBytes,
    pub script_code_hash: BsonBytes,
    pub script_args: BsonBytes,
    pub script_type: u8,
    pub script_args_len: u32,
}

#[allow(clippy::from_over_into)]
impl Into<packed::Script> for ScriptTable {
    fn into(self) -> packed::Script {
        packed::ScriptBuilder::default()
            .code_hash(
                H256::from_slice(&self.script_code_hash.bytes[0..32])
                    .unwrap()
                    .pack(),
            )
            .args(self.script_args.bytes.pack())
            .hash_type(packed::Byte::new(self.script_type))
            .build()
    }
}

impl Hash for ScriptTable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.script_hash.bytes.hash(state);
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
        let mut encode = self.script_hash.bytes.clone();
        encode.extend_from_slice(&self.script_hash_160.bytes);
        encode.extend_from_slice(&self.script_code_hash.bytes);
        encode.extend_from_slice(&self.script_args_len.to_be_bytes());
        encode.push(self.script_type);
        encode.extend_from_slice(&self.script_args.bytes);
        encode
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        ScriptTable {
            script_hash: to_bson_bytes(&bytes[0..32]),
            script_hash_160: to_bson_bytes(&bytes[32..52]),
            script_code_hash: to_bson_bytes(&bytes[52..84]),
            script_args: to_bson_bytes(&bytes[89..]),
            script_args_len: u32::from_be_bytes(to_fixed_array::<4>(&bytes[84..88])),
            script_type: bytes[88],
        }
    }
}

#[crud_table(table_name: "mercury_uncle_relationship" | formats_pg: "
    block_hash:{}::bytea,
    uncle_hashes:{}::bytea"
)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UncleRelationshipTable {
    pub block_hash: BsonBytes,
    pub uncle_hashes: BsonBytes,
}

impl UncleRelationshipTable {
    pub fn new(block_hash: BsonBytes, uncle_hashes: BsonBytes) -> Self {
        UncleRelationshipTable {
            block_hash,
            uncle_hashes,
        }
    }
}

#[crud_table(table_name: "mercury_canonical_chain" | formats_pg: "block_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalChainTable {
    pub block_number: u64,
    pub block_hash: BsonBytes,
}

impl Default for CanonicalChainTable {
    fn default() -> Self {
        CanonicalChainTable {
            block_number: 0,
            block_hash: empty_bson_bytes(),
        }
    }
}

impl PartialEq for CanonicalChainTable {
    fn eq(&self, other: &Self) -> bool {
        self.block_number == other.block_number && self.block_hash.bytes == other.block_hash.bytes
    }
}

impl Eq for CanonicalChainTable {}

impl PartialOrd for CanonicalChainTable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.block_number.partial_cmp(&other.block_number)
    }
}

impl Ord for CanonicalChainTable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_number.cmp(&other.block_number)
    }
}

impl CanonicalChainTable {
    pub fn new(block_number: u64, block_hash: BsonBytes) -> Self {
        CanonicalChainTable {
            block_number,
            block_hash,
        }
    }
}

#[crud_table(table_name: "mercury_registered_address" | formats_pg: "lock_hash:{}::bytea")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisteredAddressTable {
    pub lock_hash: BsonBytes,
    pub address: String,
}

impl RegisteredAddressTable {
    pub fn new(lock_hash: BsonBytes, address: String) -> Self {
        RegisteredAddressTable { lock_hash, address }
    }
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
            script_hash: to_bson_bytes(&rand_bytes(32)),
            script_hash_160: to_bson_bytes(&rand_bytes(20)),
            script_code_hash: to_bson_bytes(&rand_bytes(32)),
            script_args: to_bson_bytes(&args),
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
