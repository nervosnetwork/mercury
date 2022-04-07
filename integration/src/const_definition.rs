use ckb_types::{h256, H256};

pub const RPC_TRY_COUNT: usize = 10;
pub const RPC_TRY_INTERVAL_SECS: u64 = 5;

pub const CELL_BASE_MATURE_EPOCH: u64 = 4;
pub const GENESIS_EPOCH_LENGTH: u64 = 100;
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
