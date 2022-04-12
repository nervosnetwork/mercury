use arc_swap::ArcSwap;
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::packed;
use parking_lot::RwLock;

use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    pub static ref CURRENT_BLOCK_NUMBER: ArcSwap<BlockNumber> = ArcSwap::from_pointee(0u64);
    pub static ref CURRENT_EPOCH_NUMBER: ArcSwap<RationalU256> = ArcSwap::from_pointee(RationalU256::zero());
}
