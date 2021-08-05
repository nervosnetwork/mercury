use crate::table::{
    BigDataTable, BlockTable, CellTable, LiveCellTable, ScriptTable, TransactionTable,
};
use crate::XSQLPool;

use common::anyhow::Result;

use ckb_types::core::{BlockView, TransactionView};
use ckb_types::{bytes::Bytes, packed, prelude::*};
use rbatis::crud::CRUDMut;
use rbatis::executor::RBatisTxExecutor;

const BIG_DATA_THRESHOLD: usize = 1024 * 8;

impl XSQLPool {
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &BlockTable {
                block_hash: hex::encode(block_view.hash().as_slice()),
                block_number: block_view.number(),
                version: block_view.version(),
                compact_target: block_view.compact_target(),
                timestamp: block_view.timestamp(),
                epoch: block_view.epoch().full_value(),
                parent_hash: hex::encode(block_view.parent_hash().as_slice()),
                transactions_root: hex::encode(block_view.transactions_root().as_slice()),
                proposals_hash: hex::encode(block_view.proposals_hash().as_slice()),
                uncles_hash: hex::encode(block_view.uncles_hash().as_slice()),
                dao: hex::encode(block_view.dao().as_slice()),
                nonce: block_view.nonce().to_string(),
                proposals: hex::encode(block_view.data().proposals().as_slice()),
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
        let version = block_view.version();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            tx.save(
                &TransactionTable {
                    id: None,
                    tx_hash: hex::encode(transaction.hash().as_slice()),
                    tx_index: index,
                    input_count: transaction.inputs().len() as u32,
                    output_count: transaction.outputs().len() as u32,
                    cell_deps: transaction.cell_deps().as_bytes().to_vec(),
                    header_deps: transaction.header_deps().as_bytes().to_vec(),
                    witness: transaction.witnesses().as_bytes().to_vec(),
                    block_number,
                    timestamp,
                    version,
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
        let tx_hash = hex::encode(tx_view.hash().as_slice());
        let block_hash = hex::encode(block_view.hash().as_slice());
        let block_number = block_view.number();
        let epoch_number = block_view.epoch().full_value();

        // Todo: handle inputs
        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let index = idx as u32;
            let (is_data_complete, cell_data) = self.parse_cell_data(&data);
            let mut table = CellTable {
                id: None,
                tx_hash: tx_hash.clone(),
                output_index: index,
                block_hash: block_hash.clone(),
                capacity: cell.capacity().unpack(),
                lock_hash: hex::encode(cell.lock().calc_script_hash().as_slice()),
                lock_code_hash: hex::encode(cell.lock().code_hash().as_slice()),
                lock_args: hex::encode(cell.lock().args().as_slice()),
                lock_script_type: cell.lock().hash_type().into(),
                data: cell_data,
                is_data_complete,
                block_number,
                epoch_number,
                tx_index,
                ..Default::default()
            };

            self.insert_script_table(table.to_lock_script_table(), tx)
                .await?;

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                self.insert_script_table(table.to_type_script_table(), tx)
                    .await?;
            }

            if !table.is_data_complete {
                self.insert_big_data_table(tx_hash.clone(), index, data.to_vec(), tx)
                    .await?;
            }

            tx.save(&table, &[]).await?;
            self.insert_live_cell_table(table.into(), tx).await?;
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
