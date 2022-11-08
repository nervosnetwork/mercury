use common::PaginationRequest;
use core_rpc_types::{AssetInfo, Item};
use core_storage::DetailedCell;

use ckb_types::packed;

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Debug, Default)]
pub struct TransferComponents {
    pub inputs: Vec<DetailedCell>,
    pub outputs: Vec<packed::CellOutput>,
    pub outputs_data: Vec<packed::Bytes>,
    pub header_deps: Vec<packed::Byte32>,
    pub script_deps: BTreeSet<String>,
    pub type_witness_args: HashMap<usize, (packed::BytesOpt, packed::BytesOpt)>,
    pub fee_change_cell_index: Option<usize>,
    pub dao_reward_capacity: u64,
    pub dao_since_map: HashMap<usize, u64>,
    pub inputs_not_require_signature: HashSet<usize>,
}

impl TransferComponents {
    pub fn new() -> Self {
        TransferComponents::default()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PoolCkbPriority {
    DaoClaim,   // AssetType::CKB and Dao
    Normal,     // AssetType::CKB
    WithUdt,    // AssetType::UDT
    AcpFeature, // AssetType::CKB or AssetType::UDT
}

#[derive(Debug, Copy, Clone)]
pub enum PoolUdtPriority {
    Cheque,
    Normal,
    AcpFeature,
    PwLockEthereum,
}

pub struct CkbCellsCache {
    pub cell_deque: VecDeque<(DetailedCell, PoolCkbPriority, Item)>,

    pub items: Vec<Item>,
    pub item_asset_iter_plan: Vec<(usize, PoolCkbPriority)>,
    pub current_plan_index: usize,
    pub current_pagination: PaginationRequest,
}

impl CkbCellsCache {
    pub fn new(items: Vec<Item>) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[
                PoolCkbPriority::DaoClaim,
                PoolCkbPriority::Normal,
                PoolCkbPriority::WithUdt,
                PoolCkbPriority::AcpFeature,
            ] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        CkbCellsCache {
            items,
            item_asset_iter_plan: item_category_array,
            current_plan_index: 0,
            cell_deque: VecDeque::new(),
            current_pagination: PaginationRequest::default(),
        }
    }

    pub fn new_acp(items: Vec<Item>) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[PoolCkbPriority::AcpFeature] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        CkbCellsCache {
            items,
            item_asset_iter_plan: item_category_array,
            current_plan_index: 0,
            cell_deque: VecDeque::new(),
            current_pagination: PaginationRequest::default(),
        }
    }

    pub fn get_current_item_index(&self) -> usize {
        if self.current_plan_index >= self.item_asset_iter_plan.len() {
            return self.items.len();
        }
        self.item_asset_iter_plan[self.current_plan_index].0
    }
}

pub struct UdtCellsCache {
    pub cell_deque: VecDeque<(DetailedCell, PoolUdtPriority, Item)>,

    pub items: Vec<Item>,
    pub asset_info: AssetInfo,
    pub item_asset_iter_plan: Vec<(usize, PoolUdtPriority)>,
    pub current_plan_index: usize,
    pub current_pagination: PaginationRequest,
}

impl UdtCellsCache {
    pub fn new(items: Vec<Item>, asset_info: AssetInfo) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[
                PoolUdtPriority::Cheque,
                PoolUdtPriority::Normal,
                PoolUdtPriority::AcpFeature,
                PoolUdtPriority::PwLockEthereum,
            ] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        UdtCellsCache {
            items,
            asset_info,
            item_asset_iter_plan: item_category_array,
            current_plan_index: 0,
            cell_deque: VecDeque::new(),
            current_pagination: PaginationRequest::default(),
        }
    }

    pub fn new_acp(items: Vec<Item>, asset_info: AssetInfo) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[PoolUdtPriority::AcpFeature, PoolUdtPriority::PwLockEthereum] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        UdtCellsCache {
            items,
            asset_info,
            item_asset_iter_plan: item_category_array,
            current_plan_index: 0,
            cell_deque: VecDeque::new(),
            current_pagination: PaginationRequest::default(),
        }
    }
}
