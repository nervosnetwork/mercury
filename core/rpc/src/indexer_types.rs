use ckb_jsonrpc_types::{CellOutput, JsonBytes, OutPoint, Script};
use ckb_types::bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct GetCellPayload {
    pub search_key: SearchKey,
    pub order: Order,
    pub limit: u64,
    pub after_cursor: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SearchKey {
    pub script: Script,
    pub script_type: ScriptType,
    pub filter: Option<SearchKeyFilter>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SearchKeyFilter {
    pub script: Option<Script>,
    pub output_data_len_range: Option<[u64; 2]>,
    pub output_capacity_range: Option<[u64; 2]>,
    pub block_range: Option<[u64; 2]>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    Lock,
    Type,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Desc,
    Asc,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Cell {
    pub output: CellOutput,
    pub output_data: JsonBytes,
    pub out_point: OutPoint,
    pub block_number: u64,
    pub tx_index: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct PaginationResponse<T> {
    pub objects: Vec<T>,
    pub last_cursor: Option<Bytes>,
}

impl From<common::Order> for Order {
    fn from(order: common::Order) -> Order {
        match order {
            common::Order::Asc => Order::Asc,
            common::Order::Desc => Order::Asc,
        }
    }
}

impl From<Order> for common::Order {
    fn from(order: Order) -> common::Order {
        match order {
            Order::Asc => common::Order::Asc,
            Order::Desc => common::Order::Asc,
        }
    }
}

impl From<common::DetailedCell> for Cell {
    fn from(cell: common::DetailedCell) -> Cell {
        Cell {
            output: cell.cell_output.into(),
            output_data: JsonBytes::from_bytes(cell.cell_data),
            out_point: cell.out_point.into(),
            block_number: cell.block_number,
            tx_index: cell.tx_index,
        }
    }
}