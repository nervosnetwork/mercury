use common::{DetailedCell, PaginationRequest};
use core_rpc_types::{AssetInfo, Item, SignatureAction, Source};

use ckb_types::packed;

use std::collections::{HashMap, HashSet, VecDeque};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum AssetScriptType {
    Secp256k1,
    ACP,
    ChequeUnlock(String),
    ChequeInLock(String),
    Dao(Item),
    PwLock,
}

#[derive(Debug, Default)]
pub struct TransferComponents {
    pub inputs: Vec<DetailedCell>,
    pub outputs: Vec<packed::CellOutput>,
    pub outputs_data: Vec<packed::Bytes>,
    pub header_deps: Vec<packed::Byte32>,
    pub script_deps: HashSet<String>,
    pub signature_actions: HashMap<String, SignatureAction>,
    pub type_witness_args: HashMap<usize, (packed::BytesOpt, packed::BytesOpt)>,
    pub fee_change_cell_index: Option<usize>,
    pub dao_reward_capacity: u64,
    pub dao_since_map: HashMap<usize, u64>,
    pub header_dep_map: HashMap<packed::Byte32, usize>,
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
    CkbChequeInLock,
    CkbChequeUnlock,
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
    pub fn new(items: Vec<Item>, asset_info: AssetInfo, source: Source) -> Self {
        let mut item_category_array = vec![];
        match source {
            Source::Claimable => {
                for (item_index, _) in items.iter().enumerate() {
                    item_category_array
                        .push((item_index, PoolUdtCategory::CkbChequeInLock.to_owned()))
                }
            }
            Source::Free => {
                for (item_index, _) in items.iter().enumerate() {
                    for category_index in &[
                        PoolUdtCategory::CkbChequeUnlock,
                        PoolUdtCategory::CkbSecpUdt,
                        PoolUdtCategory::CkbAcp,
                        PoolUdtCategory::PwLockEthereum,
                    ] {
                        item_category_array.push((item_index, category_index.to_owned()))
                    }
                }
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
