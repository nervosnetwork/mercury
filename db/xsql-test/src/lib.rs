pub mod sql;

use crate::sql::*;
use common::anyhow::Result;
use rbatis::executor::RBatisTxExecutor;

pub async fn delete_all_data(tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
    delete_block_table_data(tx).await?;
    delete_transaction_table_data(tx).await?;
    delete_cell_table_data(tx).await?;
    delete_consume_info_table_data(tx).await?;
    delete_live_cell_table_data(tx).await?;
    delete_script_table_data(tx).await?;
    delete_canonical_chain_table_data(tx).await?;
    delete_registered_address_table_data(tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn create_tables(tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
    create_block_table(tx).await?;
    create_transaction_table(tx).await?;
    create_cell_table(tx).await?;
    create_consume_info_table(tx).await?;
    create_live_cell_table(tx).await?;
    create_script_table(tx).await?;
    create_canonical_chain_table(tx).await?;
    create_registered_address_table(tx).await?;
    tx.commit().await?;
    Ok(())
}
