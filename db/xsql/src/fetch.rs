use crate::table::BlockTable;
use crate::{DBAdapter, XSQLPool};

use common::anyhow::Result;

use ckb_types::{
    core::{BlockNumber, BlockView},
    H256,
};

use rbatis::crud::CRUD;

impl<T: DBAdapter> XSQLPool<T> {
    pub async fn get_block_by_number(&self, block_number: BlockNumber) -> Result<BlockView> {
        let _block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_number", &block_number)
            .await?;

        todo!()
    }

    pub async fn get_block_by_hash(&self, block_hash: H256) -> Result<BlockView> {
        let _block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_hash", &block_hash)
            .await?;

        todo!()
    }

    pub async fn get_tip_block(&self) -> Result<BlockView> {
        let wrapper = self.wrapper().order_by(false, &["block_number"]).limit(1);
        let _block: Option<BlockTable> = self.inner.fetch_by_wrapper(&wrapper).await?;

        todo!()
    }
}
