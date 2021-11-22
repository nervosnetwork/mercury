use common::Result;

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::H256;

pub trait Extension {
    fn append(&self, block: &BlockView) -> Result<()>;

    fn rollback(&self, tip_number: BlockNumber, tip_hash: &H256) -> Result<()>;

    fn prune(&self, tip_number: BlockNumber, tip_hash: &H256, keep_num: u64) -> Result<()>;
}
