use crate::error::DBError;
use crate::relational::table::{
    BsonBytes, CanonicalChainTable, CellTable, LiveCellTable, TransactionTable,
};
use crate::relational::{sql, RelationalStorage};

use ckb_types::prelude::Unpack;
use common::Result;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

use ckb_types::{core::BlockNumber, packed, H256};

impl RelationalStorage {
    pub(crate) async fn remove_tx_and_cell(
        &self,
        _block_number: BlockNumber,
        block_hash: BsonBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hashes = sql::get_tx_hash_by_block_hash(tx, block_hash.clone())
            .await?
            .ok_or_else(|| DBError::FetchDataError("transaction".to_string()))?;

        tx.remove_by_column::<TransactionTable, BsonBytes>("block_hash", &block_hash)
            .await?;
        tx.remove_batch_by_column::<CellTable, BsonBytes>("tx_hash", &tx_hashes)
            .await?;

        Ok(())
    }

    pub(crate) async fn remove_canonical_chain(
        &self,
        _block_number: BlockNumber,
        block_hash: BsonBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.remove_by_column::<CanonicalChainTable, BsonBytes>("block_hash", &block_hash)
            .await?;

        Ok(())
    }

    pub(crate) async fn remove_live_cell_by_out_point(
        &self,
        out_point: &packed::OutPoint,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hash: H256 = out_point.tx_hash().unpack();
        let output_index: u32 = out_point.index().unpack();

        let w = self
            .pool
            .wrapper()
            .eq("tx_hash", tx_hash)
            .and()
            .eq("output_index", output_index);
        tx.remove_by_wrapper::<LiveCellTable>(&w).await?;

        Ok(())
    }
}
