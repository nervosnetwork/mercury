use crate::table::{BsonBytes, CanonicalChainTable, CellTable, LiveCellTable, TransactionTable};
use crate::{error::DBError, sql, DBAdapter, XSQLPool};

use common::anyhow::Result;

use ckb_types::core::BlockNumber;
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

impl<T: DBAdapter> XSQLPool<T> {
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
        self.remove_cell_table(tx_hashes, tx).await?;

        Ok(())
    }

    async fn remove_cell_table(
        &self,
        tx_hashes: Vec<BsonBytes>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.remove_batch_by_column::<CellTable, BsonBytes>("tx_hash", &tx_hashes)
            .await?;
        tx.remove_batch_by_column::<LiveCellTable, BsonBytes>("tx_hash", &tx_hashes)
            .await?;

        Ok(())
    }

    pub(crate) async fn remove_canonical_chain(
        &self,
        block_number: BlockNumber,
        block_hash: BsonBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let wrapper = self
            .wrapper()
            .eq("block_hash", block_hash)
            .eq("block_number", block_number);

        tx.remove_by_wrapper::<CanonicalChainTable>(&wrapper)
            .await?;

        Ok(())
    }
}
