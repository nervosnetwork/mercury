use crate::relational::table::{BlockTable, CanonicalChainTable, LiveCellTable, TransactionTable};
use crate::relational::{empty_rb_bytes, sql, to_rb_bytes, RelationalStorage};

use common::{Context, Result};
use common_logger::tracing_async;

use ckb_types::prelude::Unpack;
use db_xsql::rbatis::core::types::byte::RbBytes;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

use ckb_types::{core::BlockNumber, packed};

impl RelationalStorage {
    pub(crate) async fn remove_tx_and_cell(
        &self,
        _ctx: Context,
        _block_number: BlockNumber,
        block_hash: RbBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hashes = sql::get_tx_hashes_by_block_hash(tx, block_hash.clone())
            .await?
            .into_iter()
            .map(|hash| hash.inner())
            .collect::<Vec<_>>();

        tx.remove_by_column::<TransactionTable, RbBytes>("block_hash", &block_hash)
            .await?;
        tx.remove_batch_by_column::<LiveCellTable, RbBytes>("tx_hash", &tx_hashes)
            .await?;

        for tx_hash in tx_hashes.iter() {
            sql::rollback_consume_cell(tx, empty_rb_bytes(), tx_hash.clone()).await?;
        }

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn remove_block_table(
        &self,
        _ctx: Context,
        _block_number: BlockNumber,
        block_hash: RbBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.remove_by_column::<BlockTable, RbBytes>("block_hash", &block_hash)
            .await?;
        tx.remove_by_column::<CanonicalChainTable, RbBytes>("block_hash", &block_hash)
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
