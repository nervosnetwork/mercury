pub mod sql;

use crate::sql::*;
use common::anyhow::Result;
use sqlx::{Any, Transaction};

pub async fn delete_all_data(mut tx: Transaction<'_, Any>) -> Result<()> {
    delete_block_table_data(&mut tx).await?;
    delete_transaction_table_data(&mut tx).await?;
    delete_cell_table_data(&mut tx).await?;
    delete_consume_info_table_data(&mut tx).await?;
    delete_live_cell_table_data(&mut tx).await?;
    delete_indexer_cell_table_data(&mut tx).await?;
    delete_script_table_data(&mut tx).await?;
    delete_canonical_chain_table_data(&mut tx).await?;
    delete_registered_address_table_data(&mut tx).await?;
    delete_sync_status_table_data(&mut tx).await?;
    tx.commit().await.map(|_| ()).map_err(Into::into)
}

pub async fn create_tables(mut tx: Transaction<'_, Any>) -> Result<()> {
    create_block_table(&mut tx).await?;
    create_transaction_table(&mut tx).await?;
    create_cell_table(&mut tx).await?;
    create_consume_info_table(&mut tx).await?;
    create_live_cell_table(&mut tx).await?;
    create_indexer_cell_table(&mut tx).await?;
    create_script_table(&mut tx).await?;
    create_canonical_chain_table(&mut tx).await?;
    create_registered_address_table(&mut tx).await?;
    create_sync_status_table(&mut tx).await?;
    tx.commit().await.map(|_| ()).map_err(Into::into)
}
