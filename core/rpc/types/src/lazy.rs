use arc_swap::ArcSwap;
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{packed, H256};
use parking_lot::RwLock;

use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref TX_POOL_CACHE: RwLock<HashSet<packed::OutPoint>> = RwLock::new(HashSet::new());
    pub static ref CURRENT_BLOCK_NUMBER: ArcSwap<BlockNumber> = ArcSwap::from_pointee(0u64);
    pub static ref CURRENT_EPOCH_NUMBER: ArcSwap<RationalU256> = ArcSwap::from_pointee(RationalU256::zero());
    pub static ref SECP256K1_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref SUDT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref ACP_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref CHEQUE_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref DAO_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref PW_LOCK_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
}
