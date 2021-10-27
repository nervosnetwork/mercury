use crate::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, ConsumedInfo, IndexerCellTable, LiveCellTable,
    RegisteredAddressTable, ScriptTable, TransactionTable,
};
use crate::relational::{generate_id, sql, to_rb_bytes, RelationalStorage};

use common::{Context, Result};
use common_logger::tracing_async;
use db_xsql::rbatis::core::types::byte::RbBytes;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor};

use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::prelude::*;

use std::collections::{HashMap, HashSet};

const BATCH_SIZE_THRESHOLD: usize = 1000;

impl RelationalStorage {
    #[tracing_async]
    pub(crate) async fn insert_block_table(
        &self,
        _ctx: Context,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());

        tx.save(&BlockTable::from(block_view), &[]).await?;
        tx.save(
            &CanonicalChainTable::new(block_view.number(), block_hash),
            &[],
        )
        .await?;

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn insert_transaction_table(
        &self,
        _ctx: Context,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let txs = block_view.transactions();
        let block_number = block_view.number();
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());
        let block_timestamp = block_view.timestamp();
        let mut output_cell_set = Vec::new();
        let mut live_cell_set = Vec::new();
        let mut tx_set = Vec::new();
        let mut script_set = HashSet::new();
        let mut consumed_infos = Vec::new();
        let mut indexer_cells = Vec::new();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            tx_set.push(TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                index,
                block_hash.clone(),
                block_number,
                block_timestamp,
            ));

            self.insert_cell_table(
                transaction,
                index,
                block_view,
                tx,
                &mut output_cell_set,
                &mut live_cell_set,
                &mut script_set,
                &mut consumed_infos,
                &mut indexer_cells,
            )
            .await?;

            if tx_set.len() >= BATCH_SIZE_THRESHOLD {
                tx.save_batch(&tx_set, &[]).await?;
                tx_set.clear();
            }
        }

        if !tx_set.is_empty() {
            tx.save_batch(&tx_set, &[]).await?;
        }

        if !output_cell_set.is_empty() {
            tx.save_batch(&output_cell_set, &[]).await?;
            tx.save_batch(&live_cell_set, &[]).await?;
        }

        if !script_set.is_empty() {
            let script_batch = script_set.iter().cloned().collect::<Vec<_>>();
            tx.save_batch(&script_batch, &[]).await?;
        }

        self.update_consumed_cells(consumed_infos, tx).await?;

        Ok(())
    }

    async fn update_input_indexer_cells(&self, block_number: u64, tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
        Ok(())
    }

    async fn update_consumed_cells(
        &self,
        infos: Vec<ConsumedInfo>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        for info in infos.iter() {
            let tx_hash = to_rb_bytes(&info.out_point.tx_hash().raw_data());
            let output_index: u32 = info.out_point.index().unpack();

            self.remove_live_cell_by_out_point(&info.out_point, tx)
                .await?;
            sql::update_consume_cell(
                tx,
                info.consumed_block_number,
                info.consumed_block_hash.clone(),
                info.consumed_tx_hash.clone(),
                info.consumed_tx_index,
                info.input_index,
                info.since.clone(),
                tx_hash,
                output_index,
            )
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
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
        consumed_infos: &mut Vec<ConsumedInfo>,
        indexer_cells: &mut Vec<IndexerCellTable>,
    ) -> Result<()> {
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());
        let block_number = block_view.number();
        let epoch = block_view.epoch();

        if tx_index > 0 {
            self.collect_consume_input_cells(
                tx_view,
                block_number,
                block_hash.clone(),
                tx_index,
                tx,
                consumed_infos,
                indexer_cells,
            )
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
            indexer_cells,
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
        block_hash: RbBytes,
        epoch: EpochNumberWithFraction,
        tx: &mut RBatisTxExecutor<'_>,
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
        indexer_cell_set: &mut Vec<IndexerCellTable>,
    ) -> Result<()> {
        let tx_hash = to_rb_bytes(&tx_view.hash().raw_data());
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
                let type_script_table = table.to_type_script_table();

                if !script_set.contains(&type_script_table)
                    && !self
                        .has_script(&type_script_table, &mut has_script_cache, tx)
                        .await?
                {
                    script_set.insert(type_script_table);
                }
            }

            let lock_table = table.to_lock_script_table();
            if !script_set.contains(&lock_table)
                && !self
                    .has_script(&lock_table, &mut has_script_cache, tx)
                    .await?
            {
                script_set.insert(lock_table);
            }

            let live_cell_table: LiveCellTable = table.clone().into(); 

            output_cell_set.push(table);
            indexer_cell_set.push(IndexerCellTable::from_live_cell_table(&live_cell_table));
            live_cell_set.push(live_cell_table);
            

            if output_cell_set.len() >= BATCH_SIZE_THRESHOLD {
                tx.save_batch(output_cell_set, &[]).await?;
                tx.save_batch(live_cell_set, &[]).await?;
                tx.save_batch(indexer_cell_set, &[]).await?;
                output_cell_set.clear();
                live_cell_set.clear();
                indexer_cell_set.clear();
            }
        }

        Ok(())
    }

    async fn collect_consume_input_cells(
        &self,
        tx_view: &TransactionView,
        consumed_block_number: u64,
        consumed_block_hash: RbBytes,
        tx_index: u32,
        tx: &mut RBatisTxExecutor<'_>,
        consumed_infos: &mut Vec<ConsumedInfo>,
        indexer_cells: &mut Vec<IndexerCellTable>,
    ) -> Result<()> {
        let consumed_tx_hash = to_rb_bytes(&tx_view.hash().raw_data());

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let since: u64 = input.since().unpack();

            consumed_infos.push(ConsumedInfo {
                out_point: input.previous_output(),
                input_index: idx as u32,
                consumed_block_hash: consumed_block_hash.clone(),
                consumed_block_number,
                consumed_tx_hash: consumed_tx_hash.clone(),
                consumed_tx_index: tx_index,
                since: to_rb_bytes(&since.to_be_bytes()),
            });

            indexer_cells.push(IndexerCellTable::new_input_cell(
                generate_id(consumed_block_number),
                consumed_block_number,
                idx as u32,
                consumed_tx_hash.clone(),
                tx_index,
            ));

            if indexer_cells.len() >= BATCH_SIZE_THRESHOLD {
                tx.save_batch(indexer_cells, &[]).await?;
                indexer_cells.clear();
            }
        }

        Ok(())
    }

    async fn has_script(
        &self,
        table: &ScriptTable,
        has_script_cache: &mut HashMap<Vec<u8>, bool>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<bool> {
        if let Some(res) = has_script_cache.get(&table.script_hash.rb_bytes) {
            if *res {
                return Ok(true);
            }
        }

        let w = self
            .pool
            .wrapper()
            .eq("script_hash", table.script_hash.clone());
        let res = tx.fetch_count_by_wrapper::<ScriptTable>(w).await?;
        let ret = res != 0;

        has_script_cache
            .entry(table.script_hash.rb_bytes.clone())
            .or_insert(ret);

        Ok(ret)
    }

    pub(crate) async fn insert_registered_address_table(
        &self,
        addresses: Vec<(RbBytes, String)>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<Vec<RbBytes>> {
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
