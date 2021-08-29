use crate::table::{
    BlockTable, BsonBytes, CanonicalChainTable, CellTable, LiveCellTable, RegisteredAddressTable,
    ScriptTable, TransactionTable, UncleRelationshipTable,
};
use crate::{generate_id, sql, to_bson_bytes, DBAdapter, XSQLPool};

use common::anyhow::Result;

use cfg_if::cfg_if;
use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::prelude::*;
use rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

use std::collections::{HashMap, HashSet};

const BATCH_SIZE_THRESHOLD: usize = 1000;

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

        tx.savepoint().await?;

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
        let mut output_cell_set = Vec::new();
        let mut live_cell_set = Vec::new();
        let mut tx_set = Vec::new();
        let mut script_set = HashSet::new();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            let table = TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                index,
                block_hash.clone(),
                block_number,
                block_timestamp,
            );
            tx_set.push(table);

            self.insert_cell_table(
                transaction,
                index,
                block_view,
                tx,
                &mut output_cell_set,
                &mut live_cell_set,
                &mut script_set,
            )
            .await?;

            tx.savepoint().await?;

            if tx_set.len() >= BATCH_SIZE_THRESHOLD {
                tx.save_batch(&tx_set, &[]).await?;
                tx_set.clear();
            }
        }

        if !tx_set.is_empty() {
            tx.save_batch(&tx_set, &[]).await?;
        }

        if !output_cell_set.is_empty() {
            self.insert_cell_table_batch(output_cell_set, live_cell_set, tx)
                .await?;
        }

        if !script_set.is_empty() {
            let script_batch = script_set.iter().cloned().collect::<Vec<_>>();
            tx.save_batch(&script_batch, &[]).await?;
        }

        Ok(())
    }

    async fn insert_cell_table(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
    ) -> Result<()> {
        let block_hash = to_bson_bytes(&block_view.hash().raw_data());
        let block_number = block_view.number();
        let epoch = block_view.epoch();

        if tx_index > 0 {
            self.consume_input_cells(tx_view, block_number, block_hash.clone(), tx_index, tx)
                .await?;
        }

        self.insert_output_cells(
            tx_view,
            tx_index,
            block_number,
            block_hash.clone(),
            epoch,
            tx,
            output_cell_set,
            live_cell_set,
            script_set,
        )
        .await?;

        if script_set.len() >= BATCH_SIZE_THRESHOLD {
            let script_batch = script_set.iter().cloned().collect::<Vec<_>>();
            tx.save_batch(&script_batch, &[]).await?;
            script_set.clear();
        }

        Ok(())
    }

    async fn insert_output_cells(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_number: u64,
        block_hash: BsonBytes,
        epoch: EpochNumberWithFraction,
        tx: &mut RBatisTxExecutor<'_>,
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
    ) -> Result<()> {
        let tx_hash = to_bson_bytes(&tx_view.hash().raw_data());
        let mut has_script_cache = HashMap::new();

        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let mut table = CellTable::from_cell(
                &cell,
                generate_id(block_number),
                tx_hash.clone(),
                idx as u32,
                tx_index,
                block_number,
                block_hash.clone(),
                epoch,
                &data,
            );

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                let type_script_table = table.to_type_script_table(generate_id(block_number));

                if !script_set.contains(&type_script_table)
                    && !self
                        .has_script(&type_script_table, &mut has_script_cache, tx)
                        .await?
                {
                    script_set.insert(type_script_table);
                }
            }

            let lock_table = table.to_lock_script_table(generate_id(block_number));
            if !script_set.contains(&lock_table)
                && !self
                    .has_script(&lock_table, &mut has_script_cache, tx)
                    .await?
            {
                script_set.insert(lock_table);
            }

            output_cell_set.push(table.clone());
            live_cell_set.push(table.into_live_cell_table());

            if output_cell_set.len() >= BATCH_SIZE_THRESHOLD {
                self.insert_cell_table_batch(output_cell_set.clone(), live_cell_set.clone(), tx)
                    .await?;
                output_cell_set.clear();
                live_cell_set.clear();
            }
        }

        Ok(())
    }

    async fn consume_input_cells(
        &self,
        tx_view: &TransactionView,
        block_number: u64,
        block_hash: BsonBytes,
        tx_index: u32,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let consumed_block_number = block_number;
        let consumed_tx_hash = to_bson_bytes(&tx_view.hash().raw_data());

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let out_point = input.previous_output();
            let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
            let output_index: u32 = out_point.index().unpack();
            let w = self
                .wrapper()
                .eq("tx_hash", tx_hash.clone())
                .and()
                .eq("output_index", output_index);
            let since: u64 = input.since().unpack();

            cfg_if! {
                if #[cfg(test)] {
                    sql::update_consume_cell_sqlite(
                        tx,
                        consumed_block_number,
                        block_hash.clone(),
                        consumed_tx_hash.clone(),
                        tx_index as u16,
                        idx as u16,
                        to_bson_bytes(&since.to_be_bytes()),
                        tx_hash,
                        output_index as u16,
                    )
                    .await?;
                } else {
                    sql::update_consume_cell(
                        tx,
                        consumed_block_number,
                        block_hash.clone(),
                        consumed_tx_hash.clone(),
                        tx_index,
                        idx as u32,
                        to_bson_bytes(&since.to_be_bytes()),
                        tx_hash,
                        output_index,
                    )
                    .await?;
                }
            }

            tx.remove_by_wrapper::<LiveCellTable>(&w).await?;
        }

        Ok(())
    }

    async fn insert_cell_table_batch(
        &self,
        cell_tables: Vec<CellTable>,
        live_cell_tables: Vec<LiveCellTable>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        tx.save_batch(&cell_tables, &[]).await?;
        tx.save_batch(&live_cell_tables, &[]).await?;
        Ok(())
    }

    async fn has_script(
        &self,
        table: &ScriptTable,
        has_script_cache: &mut HashMap<Vec<u8>, bool>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<bool> {
        if let Some(res) = has_script_cache.get(&table.script_hash.bytes) {
            if *res {
                return Ok(true);
            }
        }

        let w = self.wrapper().eq("script_hash", table.script_hash.clone());
        let res = tx.fetch_count_by_wrapper::<ScriptTable>(&w).await?;
        let ret = res != 0;

        has_script_cache
            .entry(table.script_hash.bytes.clone())
            .or_insert(ret);

        Ok(ret)
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

    pub(crate) async fn insert_registered_address_table(
        &self,
        addresses: Vec<(BsonBytes, String)>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<Vec<BsonBytes>> {
        let mut res = vec![];
        for item in addresses {
            let (lock_hash, address) = item;
            tx.save(
                &RegisteredAddressTable::new(lock_hash.clone(), address),
                &[],
            )
            .await?;
            res.push(lock_hash);
        }

        Ok(res)
    }
}
