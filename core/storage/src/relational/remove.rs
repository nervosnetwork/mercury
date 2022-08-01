use crate::relational::fetch::sqlx_param_placeholders;
use crate::relational::RelationalStorage;

use ckb_types::core::BlockNumber;
use ckb_types::H256;
use common::Result;
use sql_builder::SqlBuilder;
use sqlx::{Any, Transaction};

impl RelationalStorage {
    pub(crate) async fn remove_tx_and_cell(
        &self,
        _block_number: BlockNumber,
        block_hash: H256,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        let tx_hashes = self
            .query_transaction_hashes_by_block_hash(block_hash.as_bytes())
            .await?;

        sqlx::query(
            r#"DELETE FROM mercury_transaction 
            WHERE block_hash = $1"#,
        )
        .bind(block_hash.as_bytes())
        .execute(&mut *tx)
        .await?;

        self.remove_batch_by_tx_hashes("mercury_cell", &tx_hashes, tx)
            .await?;
        self.remove_batch_by_tx_hashes("mercury_live_cell", &tx_hashes, tx)
            .await?;
        self.remove_batch_by_tx_hashes("mercury_indexer_cell", &tx_hashes, tx)
            .await?;

        for tx_hash in tx_hashes.iter() {
            sqlx::query(
                "UPDATE mercury_cell SET
            consumed_block_hash = $1,
                consumed_block_number = NULL,
                consumed_tx_hash = $1,
                consumed_tx_index = NULL,
                input_index = NULL,
                since = $1 
                WHERE consumed_tx_hash = $1",
            )
            .bind(Vec::<u8>::new())
            .bind(tx_hash.as_bytes())
            .execute(&mut *tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn remove_block_table(
        &self,
        block_number: BlockNumber,
        block_hash: H256,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        sqlx::query(
            r#"DELETE FROM mercury_block 
        WHERE block_hash = $1"#,
        )
        .bind(block_hash.as_bytes())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"DELETE FROM mercury_canonical_chain 
        WHERE block_hash = $1"#,
        )
        .bind(block_hash.as_bytes())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"DELETE FROM mercury_sync_status 
        WHERE block_number = $1"#,
        )
        .bind(i32::try_from(block_number)?)
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    async fn remove_batch_by_tx_hashes(
        &self,
        table_name: &str,
        tx_hashes: &[H256],
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        if tx_hashes.is_empty() {
            return Ok(());
        }

        // build query str
        let mut query_builder = SqlBuilder::delete_from(table_name);
        let sql = query_builder
            .and_where_in("tx_hash", &sqlx_param_placeholders(1..tx_hashes.len())?)
            .sql()?;

        // bind
        let mut query = sqlx::query(&sql);
        for hash in tx_hashes {
            query = query.bind(hash.as_bytes());
        }

        // execute
        query.execute(tx).await?;

        Ok(())
    }
}
