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

pub fn is_secp_script(code_hash: &H256) -> bool {
    code_hash
        == SECP256K1_CODE_HASH
            .get()
            .expect("get built-in secp lock code hash")
}

pub fn is_acp_script(code_hash: &H256) -> bool {
    code_hash == ACP_CODE_HASH.get().expect("get built-in acp code hash")
}

pub fn is_pw_lock_script(code_hash: &H256) -> bool {
    code_hash
        == PW_LOCK_CODE_HASH
            .get()
            .expect("get built-in pw lock code hash")
}

pub fn is_cheque_script(code_hash: &H256) -> bool {
    code_hash
        == CHEQUE_CODE_HASH
            .get()
            .expect("get built-in cheque code hash")
}

pub fn is_sudt_script(code_hash: &H256) -> bool {
    code_hash == SUDT_CODE_HASH.get().expect("get sudt code hash")
}

pub fn is_dao_script(code_hash: &H256) -> bool {
    code_hash == DAO_CODE_HASH.get().expect("get dao code hash")
}
