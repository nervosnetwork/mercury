use crate::table::{
    BigDataTable, BlockTable, CellTable, LiveCellTable, ScriptTable, TransactionTable,
    UncleRelationshipTable,
};
use crate::{str, XSQLPool};

use common::anyhow::Result;

use ckb_types::core::{BlockView, TransactionView};
use ckb_types::{bytes::Bytes, prelude::*};
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor, sql};

const BIG_DATA_THRESHOLD: usize = 1024;

impl XSQLPool {
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = block_view.hash().raw_data().to_vec();
        let uncles_hash = str!(block_view.uncle_hashes());

        tx.save(
            &BlockTable {
                block_hash: block_hash.clone(),
                block_number: block_view.number(),
                version: block_view.version(),
                compact_target: block_view.compact_target(),
                block_timestamp: block_view.timestamp(),
                epoch: block_view.epoch().full_value(),
                parent_hash: block_view.parent_hash().raw_data().to_vec(),
                transactions_root: str!(block_view.transactions_root()),
                proposals_hash: str!(block_view.proposals_hash()),
                uncles_hash: uncles_hash.clone(),
                dao: str!(block_view.dao()),
                nonce: block_view.nonce().to_string(),
                proposals: str!(block_view.data().proposals()),
            },
            &[],
        )
        .await?;

        self.insert_uncle_relationship_table(block_hash, uncles_hash, tx)
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
        let block_hash = block_view.hash().raw_data().to_vec();
        let block_timestamp = block_view.timestamp();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            tx.save(
                &TransactionTable {
                    id: self.generate_id(),
                    tx_hash: transaction.hash().raw_data().to_vec(),
                    tx_index: index,
                    block_hash: block_hash.clone(),
                    tx_timestamp: block_timestamp,
                    input_count: transaction.inputs().len() as u32,
                    output_count: transaction.outputs().len() as u32,
                    cell_deps: transaction.cell_deps().as_bytes().to_vec(),
                    header_deps: transaction.header_deps().as_bytes().to_vec(),
                    witnesses: transaction.witnesses().as_bytes().to_vec(),
                    version: transaction.version(),
                    block_number,
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
        let block_hash = block_view.hash().raw_data().to_vec();
        let block_number = block_view.number();
        let epoch = block_view.epoch().full_value();

        let _ = self
            .consume_input_cells(tx_view, block_number, &block_hash, tx_index, tx)
            .await;
        self.insert_output_cells(tx_view, tx_index, block_number, &block_hash, epoch, tx)
            .await?;

        Ok(())
    }

    async fn insert_output_cells(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_number: u64,
        block_hash: &[u8],
        epoch_number: u64,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hash = tx_view.hash().raw_data().to_vec();

        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let index = idx as u32;
            let (is_data_complete, cell_data) = self.parse_cell_data(&data);
            let mut table = CellTable {
                id: self.generate_id(),
                tx_hash: tx_hash.clone(),
                output_index: index,
                block_hash: block_hash.to_vec(),
                capacity: cell.capacity().unpack(),
                lock_hash: cell.lock().calc_script_hash().raw_data().to_vec(),
                lock_code_hash: cell.lock().code_hash().raw_data().to_vec(),
                lock_args: cell.lock().args().raw_data().to_vec(),
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
                self.insert_script_table(table.to_type_script_table(self.generate_id()), tx)
                    .await?;
            }

            if !table.is_data_complete {
                self.insert_big_data_table(&tx_hash, index, data, tx)
                    .await?;
            }

            self.insert_script_table(table.to_lock_script_table(self.generate_id()), tx)
                .await?;

            tx.save(&table, &[]).await?;
            self.insert_live_cell_table(table.into_live_cell_table(), tx)
                .await?;
        }

        Ok(())
    }

    async fn consume_input_cells(
        &self,
        tx_view: &TransactionView,
        block_number: u64,
        block_hash: &[u8],
        tx_index: u32,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let consumed_block_number = block_number;
        let consumed_tx_hash = tx_view.hash().raw_data().to_vec();

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let out_point = input.previous_output();
            let tx_hash = out_point.tx_hash().raw_data().to_vec();
            let output_index: u32 = out_point.index().unpack();

            update_consume_cell(
                tx,
                consumed_block_number,
                block_hash.to_vec(),
                consumed_tx_hash.clone(),
                tx_index,
                idx as u32,
                input.since().unpack(),
                tx_hash.clone(),
                output_index,
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
        tx_hash: &[u8],
        output_index: u32,
        data: Bytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &BigDataTable {
                tx_hash: tx_hash.to_vec(),
                output_index,
                data: data.to_vec(),
            },
            &[],
        )
        .await?;

        Ok(())
    }

    async fn insert_uncle_relationship_table(
        &self,
        block_hash: Vec<u8>,
        uncles_hash: String,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &UncleRelationshipTable {
                block_hash,
                uncles_hash,
            },
            &[],
        )
        .await?;

        Ok(())
    }

    fn parse_cell_data(&self, data: &[u8]) -> (bool, Vec<u8>) {
        if data.is_empty() {
            (true, vec![])
        } else if data.len() > BIG_DATA_THRESHOLD {
            (false, vec![])
        } else {
            (true, data.to_vec())
        }
    }
}

#[sql(
    tx,
    "UPDATE cell SET
    consumed_block_number = $1, 
    consumed_block_hash = $2::bytea, 
    consumed_tx_hash = $3::bytea, 
    consumed_tx_index = $4, 
    input_index = $5, 
    since = $6 
    WHERE tx_hash = $7::bytea AND output_index = $8"
)]
async fn update_consume_cell(
    tx: &mut RBatisTxExecutor<'_>,
    consumed_block_number: u64,
    consumed_block_hash: Vec<u8>,
    consumed_tx_hash: Vec<u8>,
    consumed_tx_index: u32,
    input_index: u32,
    since: u64,
    tx_hash: Vec<u8>,
    output_index: u32,
) -> () {
}
