pub mod sql;

use common::anyhow::Result;
use sql::*;

use ckb_jsonrpc_types::BlockView as JsonBlockView;
use protocol::DB;
use rbatis::executor::RBatisTxExecutor;

pub async fn delete_all_data(tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
    delete_block_table_data(tx).await?;
    delete_transaction_table_data(tx).await?;
    delete_cell_table_data(tx).await?;
    delete_live_cell_table_data(tx).await?;
    delete_script_table_data(tx).await?;
    delete_uncle_relationship_table_data(tx).await?;
    delete_canonical_chain_table_data(tx).await?;
    delete_registered_address_table_data(tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn create_tables(tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
    create_block_table(tx).await?;
    create_transaction_table(tx).await?;
    create_cell_table(tx).await?;
    create_live_cell_table(tx).await?;
    create_script_table(tx).await?;
    create_uncle_relationship_table(tx).await?;
    create_canonical_chain_table(tx).await?;
    create_registered_address_table(tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn insert_blocks<T: protocol::DBAdapter>(pool: &xsql::XSQLPool<T>, block_dir: &str) {
    let data_path = String::from(block_dir);
    for i in 0..10 {
        pool.append_block(read_block_view(i, data_path.clone()).into())
            .await
            .unwrap();
    }
}

pub fn read_block_view(number: u64, dir_path: String) -> JsonBlockView {
    let file_name = number.to_string() + ".json";
    let path = dir_path + file_name.as_str();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
