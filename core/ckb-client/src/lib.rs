pub mod client;
pub mod error;

pub use client::CkbRpcClient;

use common::{async_trait, Result};
use core_synchronization::SyncAdapter;

use ckb_jsonrpc_types::{
    BlockView, EpochView, LocalNode, RawTxPool, TransactionWithStatus, Uint64,
};
use ckb_types::{core, H256};

#[async_trait]
pub trait CkbRpc: Sync + Send + 'static {
    async fn local_node_info(&self) -> Result<LocalNode>;

    async fn get_tip_block_number(&self) -> Result<u64>;

    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool>;

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>>;

    async fn get_blocks_by_number(
        &self,
        block_number: Vec<core::BlockNumber>,
    ) -> Result<Vec<Option<BlockView>>>;

    async fn get_epoch_by_number(&self, epoch_number: Uint64) -> Result<EpochView>;

    async fn get_current_epoch(&self) -> Result<EpochView>;

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>>;
}

#[async_trait]
impl SyncAdapter for dyn CkbRpc {
    async fn pull_blocks(
        &self,
        block_numbers: Vec<core::BlockNumber>,
    ) -> Result<Vec<core::BlockView>> {
        let mut ret = Vec::new();
        for (idx, block) in self
            .get_blocks_by_number(block_numbers.clone())
            .await?
            .iter()
            .enumerate()
        {
            if let Some(b) = block {
                ret.push(core::BlockView::from(b.to_owned()));
            } else {
                log::error!("[sync] Get none block {:?} from node", block_numbers[idx]);
            }
        }

        Ok(ret)
    }
}
