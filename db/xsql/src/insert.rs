use crate::table::{
    BigDataTable, BlockTable, CellTable, LiveCellTable, ScriptTable, TransactionTable,
};
use crate::{str, XSQLPool};

use common::anyhow::Result;

use ckb_types::core::{BlockView, TransactionView};
use ckb_types::prelude::*;
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};
use rbatis::{plugin::snowflake::SNOWFLAKE, sql};

const BIG_DATA_THRESHOLD: usize = 1024;

impl XSQLPool {
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &BlockTable {
                block_hash: str!(block_view.hash()),
                block_number: block_view.number(),
                version: block_view.version(),
                compact_target: block_view.compact_target(),
                timestamp: block_view.timestamp(),
                epoch: block_view.epoch().full_value(),
                parent_hash: str!(block_view.parent_hash()),
                transactions_root: str!(block_view.transactions_root()),
                proposals_hash: str!(block_view.proposals_hash()),
                uncles_hash: str!(block_view.uncles_hash()),
                dao: str!(block_view.dao()),
                nonce: block_view.nonce().to_string(),
                proposals: str!(block_view.data().proposals()),
            },
            &[],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_transaction_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let txs = block_view.transactions();
        let block_number = block_view.number();
        let timestamp = block_view.timestamp();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            tx.save(
                &TransactionTable {
                    id: SNOWFLAKE.generate(),
                    tx_hash: str!(transaction.hash()),
                    tx_index: index,
                    input_count: transaction.inputs().len() as u32,
                    output_count: transaction.outputs().len() as u32,
                    cell_deps: transaction.cell_deps().as_bytes().to_vec(),
                    header_deps: transaction.header_deps().as_bytes().to_vec(),
                    witness: transaction.witnesses().as_bytes().to_vec(),
                    version: transaction.version(),
                    block_number,
                    timestamp,
                },
                &[],
            )
            .await?;

            self.insert_cell_table(transaction, index, block_view, tx)
                .await?;
        }

        Ok(())
    }

    async fn insert_cell_table(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_number = block_view.number();
        let block_hash = str!(block_view.hash());
        let epoch = block_view.epoch().full_value();

        self.consume_input_cells(tx_view, block_number, &block_hash, tx_index, tx)
            .await?;
        self.insert_output_cells(tx_view, tx_index, block_number, &block_hash, epoch, tx)
            .await?;

        Ok(())
    }

    async fn insert_output_cells(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_number: u64,
        block_hash: &str,
        epoch_number: u64,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hash = str!(tx_view.hash());

        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let index = idx as u32;
            let (is_data_complete, cell_data) = self.parse_cell_data(&data);
            let mut table = CellTable {
                id: SNOWFLAKE.generate(),
                tx_hash: tx_hash.clone(),
                output_index: index,
                block_hash: block_hash.to_string(),
                capacity: cell.capacity().unpack(),
                lock_hash: str!(cell.lock().calc_script_hash()),
                lock_code_hash: str!(cell.lock().code_hash()),
                lock_args: str!(cell.lock().args()),
                lock_script_type: cell.lock().hash_type().into(),
                data: cell_data,
                is_data_complete,
                block_number,
                epoch_number,
                tx_index,
                ..Default::default()
            };

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                self.insert_script_table(table.to_type_script_table(), tx)
                    .await?;
            }

            if !table.is_data_complete {
                self.insert_big_data_table(tx_hash.clone(), index, data.to_vec(), tx)
                    .await?;
            }

            self.insert_script_table(table.to_lock_script_table(), tx)
                .await?;

            tx.save(&table, &[]).await?;
            self.insert_live_cell_table(table.into(), tx).await?;
        }

        Ok(())
    }

    async fn consume_input_cells(
        &self,
        tx_view: &TransactionView,
        block_number: u64,
        block_hash: &str,
        tx_index: u32,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let consumed_block_number = block_number;
        let consumed_tx_hash = str!(tx_view.hash());

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let out_point = input.previous_output();
            let tx_hash = str!(out_point.tx_hash());
            let output_index: u32 = out_point.index().unpack();

            update_consume_cell(
                tx,
                consumed_block_number,
                block_hash.to_string(),
                consumed_tx_hash.clone(),
                tx_index,
                idx as u32,
                input.since().unpack(),
                tx_hash.clone(),
                output_index,
            )
            .await?;

            // Remove cell from live cell table
            tx.remove_by_wrapper::<LiveCellTable>(
                &self
                    .wrapper()
                    .eq("tx_hash", tx_hash)
                    .and()
                    .eq("output_index", output_index),
            )
            .await?;
        }

        Ok(())
    }

    async fn insert_live_cell_table(
        &self,
        table: LiveCellTable,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(&table, &[]).await?;
        Ok(())
    }

    async fn insert_script_table(
        &self,
        table: ScriptTable,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(&table, &[]).await?;
        Ok(())
    }

    async fn insert_big_data_table(
        &self,
        tx_hash: String,
        output_index: u32,
        data: Vec<u8>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &BigDataTable {
                tx_hash,
                output_index,
                data,
            },
            &[],
        )
        .await?;

        Ok(())
    }

    fn parse_cell_data(&self, data: &[u8]) -> (bool, Option<String>) {
        if data.is_empty() {
            (true, None)
        } else if data.len() > BIG_DATA_THRESHOLD {
            (false, None)
        } else {
            (true, Some(hex::encode(data)))
        }
    }
}

#[sql(
    tx,
    "UPDATE cell SET consumed_block_number = #{consumed_block_number}, 
    consumed_block_hash = #{consumed_block_hash}, 
    consumed_tx_hash = #{consumed_tx_hash}, 
    consumed_tx_index = #{consumed_tx_index}, 
    input_index = #{input_index}, 
    since = #{since} 
    WHERE tx_hash = #{tx_hash} AND output_index = #{output_index}"
)]
async fn update_consume_cell(
    tx: &mut RBatisTxExecutor<'_>,
    consumed_block_number: u64,
    consumed_block_hash: String,
    consumed_tx_hash: String,
    consumed_tx_index: u32,
    input_index: u32,
    since: u64,
    tx_hash: String,
    output_index: u32,
) -> () {
}
