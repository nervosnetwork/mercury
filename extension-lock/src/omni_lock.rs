use crate::{dyn_async, LockScriptHandler};

pub use ckb_sdk::types::omni_lock::OmniLockWitnessLock;

use common::lazy::{DAO_CODE_HASH, SUDT_CODE_HASH};
use common::{utils::decode_udt_amount, Result, SECP256K1};
use core_rpc_types::{ExtraFilter, Identity, LockFilter, ScriptGroup};
use core_storage::RelationalStorage;
use core_storage::Storage;

use bitflags::bitflags;
use ckb_jsonrpc_types::CellDep;
use ckb_types::core::{RationalU256, ScriptHashType};
use ckb_types::packed::{Bytes, BytesOpt, Script, ScriptOpt};
use ckb_types::{bytes, prelude::*};
use ckb_types::{H160, H256};
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;
use std::ops::Range;

const DEFAULT_ACP_CKB_MIN: u8 = 0;
const DEFAULT_ACP_UDT_MIN: u8 = 0;

inventory::submit!(LockScriptHandler {
    name: "omni_lock",
    is_occupied_free,
    query_lock_scripts_by_identity,
    generate_extra_filter,
    script_to_identity,
    can_be_pooled_ckb,
    get_witness_lock_placeholder,
    insert_script_deps,
    get_acp_script,
    get_normal_script,
    is_anyone_can_pay,
});

/// OmniLock args
/// The lock argument has the following data structure:
/// 1. 21 byte auth
/// 2. 1 byte Omnilock flags
/// 3. 32 byte RC cell type ID, optional
/// 4. 2 bytes minimum ckb/udt in ACP, optional
/// 5. 8 bytes since for time lock, optional
/// 6. 32 bytes type script hash for supply, optional
#[derive(Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct OmniLockArgs {
    id: Identity,
    omni_lock_flags: OmniLockFlags,
    rc_args: Option<bytes::Bytes>,
    acp_args: Option<(u8, u8)>,
    time_lock_args: Option<bytes::Bytes>,
    supply_args: Option<bytes::Bytes>,
}

impl OmniLockArgs {
    fn get_acp_args(&self) -> Option<(u8, u8)> {
        self.acp_args
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct OmniLockFlags: u8 {
        /// off
        const OFF = 0;
        /// administrator mode, flag is 1, affected args:  RC cell type ID, affected field:omni_identity/signature in OmniLockWitnessLock
        const ADMIN = 0b0000_0001u8;
        // anyone-can-pay mode, flag is 1<<1, affected args: minimum ckb/udt in ACP
        const ACP = 0b0000_0010u8;
        /// time-lock mode, flag is 1<<2, affected args: since for timelock
        const TIME_LOCK = 0b0000_0100u8;
        /// supply mode, flag is 1<<3, affected args: type script hash for supply
        const SUPPLY = 0b0000_1000u8;
    }
}

fn _get_hash_type() -> ScriptHashType {
    ScriptHashType::Type
}

fn _get_cell_dep() -> CellDep {
    todo!()
}

fn can_be_pooled_ckb() -> bool {
    true
}

fn is_occupied_free(lock_args: &Bytes, cell_type: &ScriptOpt, cell_data: &bytes::Bytes) -> bool {
    let omni_args = if let Some(omni_args) = parse_omni_args(lock_args) {
        omni_args
    } else {
        return false;
    };

    if cell_data.is_empty() && cell_type.is_none() {
        return true;
    }

    if let Some(type_script) = cell_type.to_opt() {
        let type_code_hash: H256 = type_script.code_hash().unpack();
        // a ACP off sUDT cell with 0 udt amount should be spendable.
        if Some(&type_code_hash) == SUDT_CODE_HASH.get()
            && decode_udt_amount(cell_data) == Some(0)
            && omni_args.get_acp_args().is_none()
        {
            return true;
        }
        if Some(&type_code_hash) == DAO_CODE_HASH.get() {
            todo!()
        }
    }

    false
}

fn generate_extra_filter(type_script: Script) -> Option<ExtraFilter> {
    let type_code_hash: H256 = type_script.code_hash().unpack();
    if Some(&type_code_hash) == SUDT_CODE_HASH.get() {
        None
    } else {
        Some(ExtraFilter::Frozen)
    }
}

pub fn is_anyone_can_pay(lock_args: &Bytes) -> bool {
    if let Some(omni_lock_args) = parse_omni_args(lock_args) {
        if omni_lock_args.omni_lock_flags == OmniLockFlags::ACP {
            return true;
        }
    }
    false
}

fn _is_unlock(_lock_args: &Bytes, _tip: Option<RationalU256>) -> bool {
    todo!()
}

dyn_async! {
    async fn query_lock_scripts_by_identity<'a>(
        code_hash: &'a H256,
        identity: &'a Identity,
        lock_filter: &'a LockFilter,
        storage: &'a RelationalStorage,
    ) -> Result<Vec<Script>> {
        storage
            .get_scripts_by_partial_arg(code_hash, bytes::Bytes::from(identity.0.to_vec()), (0, 21))
            .await
            .map(|scripts| {
                scripts
                    .into_iter()
                    .filter(|script| {
                        if let Some(b) = lock_filter.is_acp {
                            b == is_anyone_can_pay(&script.args())
                        } else {
                            true
                        }
                    })
                    .collect()
            })
    }
}

fn script_to_identity(script: &Script) -> Option<Identity> {
    let lock_args = script.args();
    let flag = get_slice(&lock_args.raw_data(), 0..1)?[0].try_into().ok()?;
    let hash = H160::from_slice(get_slice(&lock_args.raw_data(), 1..21)?).ok()?;
    Some(Identity::new(flag, hash))
}

fn insert_script_deps(lock_name: &str, script_deps: &mut BTreeSet<String>) {
    script_deps.insert(lock_name.to_string());
    script_deps.insert(SECP256K1.to_string());
}

fn get_witness_lock_placeholder(_script_group: &ScriptGroup) -> BytesOpt {
    let witness_lock = OmniLockWitnessLock::new_builder()
        .signature(Some(bytes::Bytes::from(vec![0u8; 65])).pack())
        .build();
    Some(witness_lock.as_bytes()).pack()
}

pub fn get_acp_script(script: Script) -> Option<Script> {
    let mut args = script.args().raw_data()[0..21].to_vec();
    args.push(OmniLockFlags::ACP.bits());
    args.push(DEFAULT_ACP_CKB_MIN);
    args.push(DEFAULT_ACP_UDT_MIN);
    Some(
        script
            .as_builder()
            .args(args.pack())
            .hash_type(ScriptHashType::Type.into())
            .build(),
    )
}

fn get_normal_script(script: Script) -> Option<Script> {
    let mut args = script.args().raw_data()[0..21].to_vec();
    args.push(OmniLockFlags::OFF.bits());
    Some(
        script
            .as_builder()
            .args(args.pack())
            .hash_type(ScriptHashType::Type.into())
            .build(),
    )
}

fn parse_omni_args(lock_args: &Bytes) -> Option<OmniLockArgs> {
    let identity_flag = get_slice(&lock_args.raw_data(), 0..1)?[0].try_into().ok()?;
    let identity_hash = H160::from_slice(get_slice(&lock_args.raw_data(), 1..21)?).ok()?;
    let mut omni_args = OmniLockArgs {
        id: Identity::new(identity_flag, identity_hash),
        omni_lock_flags: OmniLockFlags::OFF,
        rc_args: None,
        acp_args: None,
        time_lock_args: None,
        supply_args: None,
    };
    let lock_args = lock_args.raw_data();
    let omni_flag = get_slice(&lock_args, 21..22)?[0];
    let optional_args = get_slice(&lock_args, 22..lock_args.len());

    if omni_flag == OmniLockFlags::OFF.bits && optional_args.is_none() {
        return Some(omni_args);
    }

    let mut optional_args = optional_args?.to_vec();
    for index in 0..8 {
        if omni_flag >> index & 1 == 1 {
            match index {
                0 => {
                    if optional_args.len() < 32 {
                        return None;
                    }
                    let left = optional_args.split_off(32);
                    omni_args.rc_args = Some(optional_args.into());
                    omni_args.omni_lock_flags |= OmniLockFlags::ADMIN;
                    optional_args = left;
                }
                1 => {
                    if optional_args.len() < 2 {
                        return None;
                    }
                    let left = optional_args.split_off(2);
                    omni_args.acp_args = Some((optional_args[0], optional_args[1]));
                    omni_args.omni_lock_flags |= OmniLockFlags::ACP;
                    optional_args = left;
                }
                2 => {
                    if optional_args.len() < 8 {
                        return None;
                    }
                    let left = optional_args.split_off(8);
                    omni_args.time_lock_args = Some(optional_args.into());
                    omni_args.omni_lock_flags |= OmniLockFlags::TIME_LOCK;
                    optional_args = left;
                }
                3 => {
                    if optional_args.len() < 32 {
                        return None;
                    }
                    let left = optional_args.split_off(32);
                    omni_args.supply_args = Some(optional_args.into());
                    omni_args.omni_lock_flags |= OmniLockFlags::SUPPLY;
                    optional_args = left;
                }
                _ => return None,
            }
        }
    }
    Some(omni_args)
}

fn get_slice(s: &[u8], range: Range<usize>) -> Option<&[u8]> {
    if s.len() > range.start && s.len() >= range.end {
        Some(&s[range])
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ckb_types::H160;

    #[test]
    fn test_parse_omni_args() {
        let mut args = vec![];
        let flag = 0u8; // IdentityFlag::Ckb
        let mut hash = H160::default().as_bytes().to_vec();
        args.push(flag);
        args.append(&mut hash);
        args.push(0b0000_0010u8);
        args.push(DEFAULT_ACP_CKB_MIN);
        args.push(DEFAULT_ACP_UDT_MIN);
        let args = parse_omni_args(&args.pack()).unwrap();
        assert!(args.acp_args.is_some());
        assert_eq!(args.acp_args.unwrap(), (0, 0));

        let mut args = vec![];
        let flag = 0u8; // IdentityFlag::Ckb
        let mut hash = H160::default().as_bytes().to_vec();
        args.push(flag);
        args.append(&mut hash);
        args.push(0b0000_0011u8);
        args.append(&mut H256::default().as_bytes().to_vec());
        args.push(DEFAULT_ACP_CKB_MIN);
        args.push(DEFAULT_ACP_UDT_MIN);
        let args = parse_omni_args(&args.pack()).unwrap();
        assert!(args.rc_args.is_some());
        assert!(args.acp_args.is_some());
        assert_eq!(args.acp_args.unwrap(), (0, 0));
        assert_eq!(args.omni_lock_flags.bits, 0b0000_0011u8);
        assert_eq!(
            args.omni_lock_flags,
            OmniLockFlags::ACP | OmniLockFlags::ADMIN
        );

        let mut args = vec![];
        let flag = 0u8; // IdentityFlag::Ckb
        let mut hash = H160::default().as_bytes().to_vec();
        args.push(flag);
        args.append(&mut hash);
        args.push(0b0000_0111u8);
        args.append(&mut H256::default().as_bytes().to_vec());
        args.push(DEFAULT_ACP_CKB_MIN);
        args.push(DEFAULT_ACP_UDT_MIN);
        args.append(&mut u64::MAX.to_le_bytes().to_vec());
        let args = parse_omni_args(&args.pack()).unwrap();
        assert!(args.rc_args.is_some());
        assert!(args.acp_args.is_some());
        assert_eq!(args.acp_args.unwrap(), (0, 0));
        assert_eq!(args.omni_lock_flags.bits, 0b0000_0111u8);
        assert_eq!(
            args.omni_lock_flags,
            OmniLockFlags::ACP | OmniLockFlags::ADMIN | OmniLockFlags::TIME_LOCK
        );

        let mut args = vec![];
        let flag = 0u8; // IdentityFlag::Ckb
        let mut hash = H160::default().as_bytes().to_vec();
        args.push(flag);
        args.append(&mut hash);
        args.push(0b0000_1111u8);
        args.append(&mut H256::default().as_bytes().to_vec());
        args.push(DEFAULT_ACP_CKB_MIN);
        args.push(DEFAULT_ACP_UDT_MIN);
        args.append(&mut u64::MAX.to_le_bytes().to_vec());
        args.append(&mut H256::default().as_bytes().to_vec());
        let args = parse_omni_args(&args.pack()).unwrap();
        assert!(args.rc_args.is_some());
        assert!(args.acp_args.is_some());
        assert_eq!(args.acp_args.unwrap(), (0, 0));
        assert_eq!(args.omni_lock_flags.bits, 0b0000_1111u8);
        assert_eq!(
            args.omni_lock_flags,
            OmniLockFlags::ACP
                | OmniLockFlags::ADMIN
                | OmniLockFlags::TIME_LOCK
                | OmniLockFlags::SUPPLY
        );

        let mut args = vec![];
        let flag = 0u8; // IdentityFlag::Ckb
        let mut hash = H160::default().as_bytes().to_vec();
        args.push(flag);
        args.append(&mut hash);
        args.push(0b0000_1000u8);
        args.append(&mut H256::default().as_bytes().to_vec());
        let args = parse_omni_args(&args.pack()).unwrap();
        assert!(args.rc_args.is_none());
        assert!(args.acp_args.is_none());
        assert!(args.time_lock_args.is_none());
        assert_eq!(args.omni_lock_flags.bits, 0b0000_1000u8);
        assert_eq!(args.omni_lock_flags, OmniLockFlags::SUPPLY);
    }
}
