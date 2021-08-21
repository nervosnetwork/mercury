use crate::table::{
    BlockTable, BsonBytes, CanonicalChainTable, CellTable, LiveCellTable, ScriptTable,
    TransactionTable, UncleRelationshipTable,
};
use crate::{generate_id, sql, to_bson_bytes, DBAdapter, XSQLPool};

use common::anyhow::Result;

use ckb_types::core::{BlockView, TransactionView};
use ckb_types::prelude::*;
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

impl<T: DBAdapter> XSQLPool<T> {
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = to_bson_bytes(&block_view.hash().raw_data());
        let uncle_hashes = to_bson_bytes(&block_view.uncle_hashes().as_bytes());
        let table: BlockTable = block_view.into();

        tx.save(&table, &[]).await?;
        self.insert_uncle_relationship_table(block_hash.clone(), uncle_hashes, tx)
            .await?;
        self.insert_cannoical_chain_table(block_view.number(), block_hash, tx)
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
                &TransactionTable::from_view(
                    transaction,
                    generate_id(block_number),
                    idx as u16,
                    block_hash.clone(),
                    block_number,
                    block_timestamp,
                ),
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

        self.consume_input_cells(tx_view, block_number, block_hash.clone(), tx_index, tx)
            .await?;
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
            let mut table = CellTable::from_cell(
                &cell,
                generate_id(block_number),
                tx_hash.clone(),
                idx as u16,
                tx_index,
                block_number,
                block_hash.clone(),
                epoch_number,
                &data,
            );

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                self.insert_script_table(table.to_type_script_table(generate_id(block_number)), tx)
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
        if sql::has_script_hash(tx, table.script_hash.clone())
            .await?
            .is_none()
        {
            tx.save(&table, &[]).await?;
        }

        Ok(())
    }

    async fn insert_uncle_relationship_table(
        &self,
        block_hash: BsonBytes,
        uncle_hashes: BsonBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(&UncleRelationshipTable::new(block_hash, uncle_hashes), &[])
            .await?;

        Ok(())
    }

    async fn insert_cannoical_chain_table(
        &self,
        block_number: u64,
        block_hash: BsonBytes,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save(&CanonicalChainTable::new(block_number, block_hash), &[])
            .await?;

        Ok(())
    }
}
