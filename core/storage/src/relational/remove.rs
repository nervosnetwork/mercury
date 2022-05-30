use crate::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, IndexerCellTable, LiveCellTable, SyncStatus,
    TransactionTable,
};
use crate::relational::{empty_rb_bytes, sql, to_rb_bytes, RelationalStorage};

use ckb_types::prelude::Unpack;
use ckb_types::{core::BlockNumber, packed};
use common::{Context, Result};
use common_logger::tracing_async;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor, Bytes as RbBytes};

impl RelationalStorage {
    pub(crate) async fn remove_tx_and_cell(
        &self,
        _ctx: Context,
        _block_number: BlockNumber,
        block_hash: RbBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hashes = sql::get_tx_hashes_by_block_hash(tx, &block_hash)
            .await?
            .into_iter()
            .map(|hash| hash.inner())
            .collect::<Vec<_>>();

        tx.remove_by_column::<TransactionTable, RbBytes>("block_hash", block_hash)
            .await?;
        tx.remove_batch_by_column::<CellTable, RbBytes>("tx_hash", &tx_hashes)
            .await?;
        tx.remove_batch_by_column::<LiveCellTable, RbBytes>("tx_hash", &tx_hashes)
            .await?;
        tx.remove_batch_by_column::<IndexerCellTable, RbBytes>("tx_hash", &tx_hashes)
            .await?;

        for tx_hash in tx_hashes.iter() {
            sql::rollback_consume_cell(tx, &empty_rb_bytes(), tx_hash).await?;
        }

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn remove_block_table(
        &self,
        _ctx: Context,
        block_number: BlockNumber,
        block_hash: RbBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.remove_by_column::<BlockTable, RbBytes>("block_hash", block_hash.clone())
            .await?;
        tx.remove_by_column::<CanonicalChainTable, RbBytes>("block_hash", block_hash)
            .await?;
        tx.remove_by_column::<SyncStatus, u64>("block_number", block_number)
            .await?;
        Ok(())
    }

    pub(crate) async fn remove_live_cell_by_out_point(
        &self,
        out_point: &packed::OutPoint,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hash = to_rb_bytes(&out_point.tx_hash().raw_data());
        let output_index: u32 = out_point.index().unpack();

        let w = self
            .pool
            .wrapper()
            .eq("tx_hash", tx_hash)
            .and()
            .eq("output_index", output_index);
        tx.remove_by_wrapper::<LiveCellTable>(w).await?;

        Ok(())
    }
}
