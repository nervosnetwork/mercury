use common::{DetailedCell, PaginationRequest};
use core_rpc_types::{AssetInfo, Item};

use ckb_types::packed;

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum AssetScriptType {
    Secp256k1,
    ACP,
    Cheque(Item),
    Dao(Item),
    PwLock,
}

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
pub enum PoolCkbCategory {
    DaoClaim,
    CkbCellBase,
    CkbAcp,
    CkbNormalSecp,
    CkbSecpUdt,
    PwLockEthereum,
}

#[derive(Debug, Copy, Clone)]
pub enum PoolUdtCategory {
    CkbCheque,
    CkbSecpUdt,
    CkbAcp,
    PwLockEthereum,
}

#[derive(Debug, Copy, Clone)]
pub enum PoolAcpCategory {
    CkbAcp,
    PwLockEthereum,
}

pub struct CkbCellsCache {
    pub items: Vec<Item>,
    pub item_category_array: Vec<(usize, PoolCkbCategory)>,
    pub array_index: usize,
    pub cell_deque: VecDeque<(DetailedCell, AssetScriptType)>,
    pub pagination: PaginationRequest,
}

impl CkbCellsCache {
    pub fn new(items: Vec<Item>) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[
                PoolCkbCategory::DaoClaim,
                PoolCkbCategory::CkbCellBase,
                PoolCkbCategory::CkbNormalSecp,
                PoolCkbCategory::CkbSecpUdt,
                PoolCkbCategory::CkbAcp,
                PoolCkbCategory::PwLockEthereum,
            ] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        CkbCellsCache {
            items,
            item_category_array,
            array_index: 0,
            cell_deque: VecDeque::new(),
            pagination: PaginationRequest::default(),
        }
    }
}

pub struct UdtCellsCache {
    pub items: Vec<Item>,
    pub asset_info: AssetInfo,
    pub item_category_array: Vec<(usize, PoolUdtCategory)>,
    pub array_index: usize,
    pub cell_deque: VecDeque<(DetailedCell, AssetScriptType)>,
    pub pagination: PaginationRequest,
}

impl UdtCellsCache {
    pub fn new(items: Vec<Item>, asset_info: AssetInfo) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[
                PoolUdtCategory::CkbCheque,
                PoolUdtCategory::CkbSecpUdt,
                PoolUdtCategory::CkbAcp,
                PoolUdtCategory::PwLockEthereum,
            ] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        UdtCellsCache {
            items,
            asset_info,
            item_category_array,
            array_index: 0,
            cell_deque: VecDeque::new(),
            pagination: PaginationRequest::default(),
        }
    }
}

pub struct AcpCellsCache {
    pub items: Vec<Item>,
    pub asset_info: Option<AssetInfo>,
    pub item_category_array: Vec<(usize, PoolAcpCategory)>,
    pub array_index: usize,
    pub cell_deque: VecDeque<(DetailedCell, AssetScriptType)>,
    pub pagination: PaginationRequest,
}

impl AcpCellsCache {
    pub fn new(items: Vec<Item>, asset_info: Option<AssetInfo>) -> Self {
        let mut item_category_array = vec![];
        for (item_index, _) in items.iter().enumerate() {
            for category_index in &[PoolAcpCategory::CkbAcp, PoolAcpCategory::PwLockEthereum] {
                item_category_array.push((item_index, category_index.to_owned()))
            }
        }
        AcpCellsCache {
            items,
            asset_info,
            item_category_array,
            array_index: 0,
            cell_deque: VecDeque::new(),
            pagination: PaginationRequest::default(),
        }
    }
}
