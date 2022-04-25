use ckb_types::{h256, H256};
use common::Address;
use once_cell::sync::OnceCell;

pub const RPC_TRY_COUNT: usize = 10;
pub const RPC_TRY_INTERVAL_SECS: u64 = 5;

pub const CELL_BASE_MATURE_EPOCH: u64 = 4;
pub const GENESIS_EPOCH_LENGTH: u64 = 10;
pub const CHEQUE_LOCK_EPOCH: u64 = 6;

pub const CKB_URI: &str = "http://127.0.0.1:8114";
pub const MERCURY_URI: &str = "http://127.0.0.1:8116";

pub const GENESIS_BUILT_IN_ADDRESS_1: &str = "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqwgx292hnvmn68xf779vmzrshpmm6epn4c0cgwga";
pub const GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY: H256 =
    h256!("0xd00c06bfd800d27397002dca6fb0993d5ba6399b4238b2f29ee9deb97593d2bc");

pub const SIGHASH_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
pub const SUDT_DEVNET_TYPE_HASH: H256 =
    h256!("0x9c6933d977360f115a3e9cd5a2e0e475853681b80d775d93ad0f8969da343e56");
pub const CHEQUE_DEVNET_TYPE_HASH: H256 =
    h256!("0x1a1e4fef34f5982906f745b048fe7b1089647e82346074e0f32c2ece26cf6b1e");
pub const ANYONE_CAN_PAY_DEVNET_TYPE_HASH: H256 =
    h256!("0x6283a479a3cf5d4276cd93594de9f1827ab9b55c7b05b3d28e4c2e0a696cfefd");
pub const DAO_DEVNET_TYPE_HASH: H256 =
    h256!("0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e");
pub const PW_LOCK_DEVNET_TYPE_HASH: H256 =
    h256!("0xe09352af0066f3162287763ce4ddba9af6bfaeab198dc7ab37f8c71c9e68bb5b");

pub static UDT_1_HASH: OnceCell<H256> = OnceCell::new();
pub static UDT_1_HOLDER_ACP_ADDRESS: OnceCell<Address> = OnceCell::new();
pub static UDT_1_HOLDER_ACP_ADDRESS_PK: OnceCell<H256> = OnceCell::new();
