use crate::utils::ScriptInfo;

use ckb_types::H256;
use once_cell::sync::OnceCell;

use std::collections::HashMap;

// built-in locks
pub static SECP256K1_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static ACP_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static PW_LOCK_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static CHEQUE_CODE_HASH: OnceCell<H256> = OnceCell::new();

// built-in types
pub static SUDT_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static DAO_CODE_HASH: OnceCell<H256> = OnceCell::new();

// These EXTENSION prefixed variables are depended on by extension lock scripts.
// Considering compatibility, please be careful if you need to modify them.
pub static EXTENSION_LOCK_SCRIPT_NAMES: OnceCell<HashMap<H256, String>> = OnceCell::new();
pub static EXTENSION_LOCK_SCRIPT_INFOS: OnceCell<HashMap<String, ScriptInfo>> = OnceCell::new();
