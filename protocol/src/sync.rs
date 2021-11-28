use common::{async_trait, Range, Result};

use ckb_types::core::BlockView;

#[async_trait]
pub trait SyncStorage: Clone + Sync + Send + 'static {
    async fn store_metadata_tables(&self, blocks: Vec<BlockView>) -> Result<()>;

    async fn updata_consume_info(&self) -> Result<()>;

    async fn build_live_cell_table(&self) -> Result<()>;

    async fn build_script_table(&self) -> Result<()>;

    async fn build_indexer_cells(&self, range: Range) -> Result<()>;

    async fn get_sync_metadata_status_by_id(&self, id: u64) -> Result<u64>;

    async fn get_sync_indexer_cell_status_by_id(&self, id: u64) -> Result<u64>;
}
