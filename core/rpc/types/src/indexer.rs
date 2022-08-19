use crate::{
    uints::{Uint32, Uint64},
    IOType,
};

use ckb_jsonrpc_types::{BlockNumber, Capacity, CellOutput, JsonBytes, OutPoint, Script};
use ckb_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SearchKey {
    pub script: Script,
    pub script_type: ScriptType,
    pub filter: Option<SearchKeyFilter>,
    pub with_data: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SearchKeyFilter {
    pub script: Option<Script>,
    pub output_data_len_range: Option<[Uint64; 2]>,
    pub output_capacity_range: Option<[Uint64; 2]>,
    pub block_range: Option<[BlockNumber; 2]>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum ScriptType {
    #[serde(alias = "lock")]
    Lock,
    #[serde(alias = "type")]
    Type,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Cell {
    pub output: CellOutput,
    pub output_data: Option<JsonBytes>,
    pub out_point: OutPoint,
    pub block_number: BlockNumber,
    pub tx_index: Uint32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationResponse<T> {
    pub objects: Vec<T>,
    pub last_cursor: Option<Uint64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Tip {
    pub block_hash: H256,
    pub block_number: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CellsCapacity {
    pub capacity: Capacity,
    pub block_hash: H256,
    pub block_number: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Transaction {
    pub tx_hash: H256,
    pub block_number: BlockNumber,
    pub tx_index: Uint32,
    pub io_index: Uint32,
    pub io_type: IOType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LiveCell {
    pub created_by: TransactionPoint,
    pub cell_output: CellOutput,
    pub output_data_len: Uint64,
    pub cellbase: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransactionPoint {
    pub block_number: BlockNumber,
    pub tx_hash: H256,
    pub index: Uint32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LockHashCapacity {
    pub capacity: Capacity,
    pub cells_count: Uint64,
    pub block_number: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CellTransaction {
    pub created_by: TransactionPoint,
    pub consumed_by: Option<TransactionPoint>,
}
