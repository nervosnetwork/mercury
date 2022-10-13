use crate::NetworkType;

use ckb_types::H256;
use once_cell::sync::OnceCell;

pub static SECP256K1_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static SUDT_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static ACP_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static CHEQUE_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static DAO_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static PW_LOCK_CODE_HASH: OnceCell<H256> = OnceCell::new();

// This NETWORK_TYPE is depended on by the lock extension plugin.
// Considering compatibility, please be careful if you need to modify it.
pub static NETWORK_TYPE: OnceCell<NetworkType> = OnceCell::new();
