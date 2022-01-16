use common::{Address, DetailedCell, PaginationRequest};
use core_rpc_types::{AssetInfo, Item, SignatureAction, Source};

use ckb_types::packed;

use std::collections::{HashMap, HashSet, VecDeque};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum AssetScriptType {
    Secp256k1,
    ACP,
    ChequeSender(String),
    ChequeReceiver(String),
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
    CkbDaoClaim,
    CkbCellBase,
    CkbAcp,
    CkbNormalSecp,
    CkbSecpUdt,
    PwLockEthereum,
}

#[derive(Debug, Copy, Clone)]
pub enum PoolUdtCategory {
    CkbChequeInTime,
    CkbChequeOutTime,
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
    pub fn new(item_and_default_address_list: Vec<(Item, Address)>) -> Self {
        let mut item_category_array = vec![];
        for (item_index, (_, default_address)) in item_and_default_address_list.iter().enumerate() {
            if default_address.is_secp256k1() {
                for category_index in &[
                    PoolCkbCategory::CkbDaoClaim,
                    PoolCkbCategory::CkbCellBase,
                    PoolCkbCategory::CkbNormalSecp,
                    PoolCkbCategory::CkbSecpUdt,
                    PoolCkbCategory::CkbAcp,
                ] {
                    item_category_array.push((item_index, category_index.to_owned()))
                }
            }
            if default_address.is_pw_lock() {
                item_category_array.push((item_index, PoolCkbCategory::PwLockEthereum.to_owned()))
            }
        }
        CkbCellsCache {
            items: item_and_default_address_list
                .into_iter()
                .map(|(item, _)| item)
                .collect(),
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
    pub fn new(
        item_and_default_address_list: Vec<(Item, Address)>,
        asset_info: AssetInfo,
        source: Source,
    ) -> Self {
        let mut item_category_array = vec![];
        match source {
            Source::Claimable => {
                for (item_index, (_, default_address)) in
                    item_and_default_address_list.iter().enumerate()
                {
                    if default_address.is_secp256k1() {
                        item_category_array
                            .push((item_index, PoolUdtCategory::CkbChequeInTime.to_owned()))
                    }
                }
            }
            Source::Free => {
                for (item_index, (_, default_address)) in
                    item_and_default_address_list.iter().enumerate()
                {
                    if default_address.is_secp256k1() {
                        for category_index in &[
                            PoolUdtCategory::CkbChequeOutTime,
                            PoolUdtCategory::CkbSecpUdt,
                            PoolUdtCategory::CkbAcp,
                        ] {
                            item_category_array.push((item_index, category_index.to_owned()))
                        }
                    }
                    if default_address.is_pw_lock() {
                        item_category_array
                            .push((item_index, PoolUdtCategory::PwLockEthereum.to_owned()))
                    }
                }
            }
        }

        UdtCellsCache {
            items: item_and_default_address_list
                .into_iter()
                .map(|(item, _)| item)
                .collect(),
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
    pub fn new(
        item_and_default_address_list: Vec<(Item, Address)>,
        asset_info: Option<AssetInfo>,
    ) -> Self {
        let mut item_category_array = vec![];

        for (item_index, (_, default_address)) in item_and_default_address_list.iter().enumerate() {
            if default_address.is_secp256k1() {
                item_category_array.push((item_index, PoolAcpCategory::CkbAcp.to_owned()))
            }
            if default_address.is_pw_lock() {
                item_category_array.push((item_index, PoolAcpCategory::PwLockEthereum.to_owned()))
            }
        }

        AcpCellsCache {
            items: item_and_default_address_list
                .into_iter()
                .map(|(item, _)| item)
                .collect(),
            asset_info,
            item_category_array,
            array_index: 0,
            cell_deque: VecDeque::new(),
            pagination: PaginationRequest::default(),
        }
    }
}
