use ckb_types::{h256, H256};

pub const RPC_TRY_COUNT: usize = 10;
pub const RPC_TRY_INTERVAL_SECS: u64 = 5;

pub const CELL_BASE_MATURE_EPOCH: u64 = 4;
pub const GENESIS_EPOCH_LENGTH: u64 = 100;

pub const CKB_URI: &str = "http://127.0.0.1:8114";
pub const MERCURY_URI: &str = "http://127.0.0.1:8116";

pub const GENESIS_BUILT_IN_ADDRESS_1: &str = "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqwgx292hnvmn68xf779vmzrshpmm6epn4c0cgwga";
pub const GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY: H256 =
    h256!("0xd00c06bfd800d27397002dca6fb0993d5ba6399b4238b2f29ee9deb97593d2bc");
