use crate::table::{
    BigDataTable, BlockTable, BsonBytes, CellTable, LiveCellTable, ScriptTable, TransactionTable,
    UncleRelationshipTable,
};
use crate::{generate_id, sql, to_bson_bytes, DBAdapter, XSQLPool};

use common::anyhow::Result;

use ckb_types::core::{BlockView, TransactionView};
use ckb_types::{bytes::Bytes, prelude::*};
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

pub const BIG_DATA_THRESHOLD: usize = 1024;

impl<T: DBAdapter> XSQLPool<T> {
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = to_bson_bytes(&block_view.hash().raw_data());
        let uncles_hash = to_bson_bytes(&block_view.uncles_hash().raw_data());
        let epoch = block_view.epoch();

        tx.save(
            &BlockTable {
                block_hash: block_hash.clone(),
                block_number: block_view.number(),
                version: block_view.version() as u16,
                compact_target: block_view.compact_target(),
                block_timestamp: block_view.timestamp(),
                epoch_number: epoch.number(),
                epoch_block_index: epoch.index() as u16,
                epoch_length: epoch.length() as u16,
                parent_hash: to_bson_bytes(&block_view.parent_hash().raw_data()),
                transactions_root: to_bson_bytes(&block_view.transactions_root().raw_data()),
                proposals_hash: to_bson_bytes(&block_view.proposals_hash().raw_data()),
                uncles_hash: uncles_hash.clone(),
                dao: to_bson_bytes(&block_view.dao().raw_data()),
                nonce: to_bson_bytes(&block_view.nonce().to_be_bytes().to_vec()),
                proposals: to_bson_bytes(&block_view.data().proposals().as_bytes().to_vec()),
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
        let block_hash = to_bson_bytes(&block_view.hash().raw_data());
        let block_timestamp = block_view.timestamp();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u16;

            tx.save(
                &TransactionTable {
                    id: generate_id(block_number),
                    tx_hash: to_bson_bytes(&transaction.hash().raw_data()),
                    tx_index: index,
                    block_hash: block_hash.clone(),
                    tx_timestamp: block_timestamp,
                    input_count: transaction.inputs().len() as u16,
                    output_count: transaction.outputs().len() as u16,
                    cell_deps: to_bson_bytes(&transaction.cell_deps().as_bytes().to_vec()),
                    header_deps: to_bson_bytes(&transaction.header_deps().as_bytes().to_vec()),
                    witnesses: to_bson_bytes(&transaction.witnesses().as_bytes().to_vec()),
                    version: transaction.version() as u16,
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
        tx_index: u16,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = to_bson_bytes(&block_view.hash().raw_data());
        let block_number = block_view.number();
        let epoch = block_view.epoch().full_value();

        let _ = self
            .consume_input_cells(tx_view, block_number, block_hash.clone(), tx_index, tx)
            .await;
        self.insert_output_cells(
            tx_view,
            tx_index,
            block_number,
            block_hash.clone(),
            epoch,
            tx,
        )
        .await?;

        Ok(())
    }

    async fn insert_output_cells(
        &self,
        tx_view: &TransactionView,
        tx_index: u16,
        block_number: u64,
        block_hash: BsonBytes,
        epoch_number: u64,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let tx_hash = to_bson_bytes(&tx_view.hash().raw_data());

        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let index = idx as u16;
            let (is_data_complete, cell_data) = self.parse_cell_data(&data);
            let mut table = CellTable::from_cell(
                &cell,
                generate_id(block_number),
                tx_hash.clone(),
                idx as u16,
                tx_index,
                block_number,
                block_hash.clone(),
                epoch_number,
            );

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                self.insert_script_table(table.to_type_script_table(generate_id(block_number)), tx)
                    .await?;
            }

            if !table.is_data_complete {
                self.insert_big_data_table(tx_hash.clone(), index, data, tx)
                    .await?;
            }

            self.insert_script_table(table.to_lock_script_table(generate_id(block_number)), tx)
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
        block_hash: BsonBytes,
        tx_index: u16,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let consumed_block_number = block_number;
        let consumed_tx_hash = to_bson_bytes(&tx_view.hash().raw_data());

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let out_point = input.previous_output();
            let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
            let output_index: u32 = out_point.index().unpack();

            sql::update_consume_cell(
                tx,
                consumed_block_number,
                block_hash.clone(),
                consumed_tx_hash.clone(),
                tx_index,
                idx as u16,
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
        let mut conn = self.acquire().await?;
        if sql::has_script_hash(&mut conn, table.script_hash.clone())
            .await?
            .is_none()
        {
            tx.save(&table, &[]).await?;
        }

        Ok(())
    }

    async fn insert_big_data_table(
        &self,
        tx_hash: BsonBytes,
        output_index: u16,
        data: Bytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(
            &BigDataTable {
                tx_hash,
                output_index,
                data: to_bson_bytes(&data),
            },
            &[],
        )
        .await?;

        Ok(())
    }

    async fn insert_uncle_relationship_table(
        &self,
        block_hash: BsonBytes,
        uncles_hash: BsonBytes,
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
