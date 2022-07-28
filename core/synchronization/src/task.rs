use crate::table::{CellTable, IndexerCellTable, SyncStatus};
use crate::{add_one_task, free_one_task, SyncAdapter, TASK_LEN};

use common::{anyhow::anyhow, Result};
use core_storage::relational::{
    bulk_insert_blocks, bulk_insert_output_cells, bulk_insert_transactions, generate_id,
    push_values_placeholders, BATCH_SIZE_THRESHOLD, IO_TYPE_INPUT, IO_TYPE_OUTPUT,
};
use db_sqlx::SQLXPool;
use db_xsql::{commit_transaction, rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::{
    core::{BlockView, TransactionView},
    prelude::*,
};
use seq_macro::seq;
use sql_builder::SqlBuilder;
use sqlx::{Any, Row, Transaction};
use tokio::time::sleep;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

const PULL_BLOCK_BATCH_SIZE: u64 = 10;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskType {
    SyncMetadata = 0u8,
    SyncIndexerCell = 1u8,
}

impl TaskType {
    fn is_metadata_task(&self) -> bool {
        *self == TaskType::SyncMetadata
    }
}

#[derive(Clone, Debug)]
pub struct Task<T> {
    id: u64,
    tip: u64,
    store: XSQLPool,
    pool: SQLXPool,
    type_: TaskType,
    state_cursor: Option<u64>,

    adapter: Arc<T>,
}

impl<T: SyncAdapter> Task<T> {
    pub fn new(
        id: u64,
        tip: u64,
        store: XSQLPool,
        pool: SQLXPool,
        adapter: Arc<T>,
        type_: TaskType,
    ) -> Task<T> {
        Task {
            id,
            tip,
            store,
            pool,
            type_,
            state_cursor: None,
            adapter,
        }
    }

    async fn set_state_cursor(&mut self) -> Result<()> {
        let last = self.last_number();

        let mut query = SqlBuilder::select_from(if self.type_.is_metadata_task() {
            "mercury_block"
        } else {
            "mercury_sync_status"
        });
        query
            .field("block_number")
            .and_where_between("block_number", self.id, last)
            .order_by("block_number", true)
            .limit(1);
        let sql = query.sql()?;
        let query = SQLXPool::new_query(&sql);
        let row = self.pool.fetch_optional(query).await?;
        let cursor = row.map_or_else(
            || self.id,
            |row| (row.get::<i64, _>("block_number") as u64 + 1).min(last),
        );

        self.state_cursor = Some(cursor);
        Ok(())
    }

    pub async fn check_done(&mut self) -> Result<bool> {
        self.set_state_cursor().await?;
        let max_number = self.state_cursor.unwrap();

        Ok(max_number == self.last_number())
    }

    pub async fn sync_metadata_process(mut self) -> Result<()> {
        if !self.type_.is_metadata_task() {
            return Err(anyhow!("{:?} Task type mismatch", self.type_));
        }

        add_one_task();

        if self.state_cursor.is_none() {
            self.set_state_cursor().await?;
        }

        let cursor = self.state_cursor.unwrap();
        let last = self.last_number();

        log::info!(
            "[sync] Sync metadata task {:?}, sync from {:?} to {:?}",
            self.id,
            cursor,
            last
        );

        for start in (cursor..=last).step_by(PULL_BLOCK_BATCH_SIZE as usize) {
            let end = (start + PULL_BLOCK_BATCH_SIZE).min(last + 1);
            let sub_task = (start..end).collect();
            let blocks = self.poll_call(Self::pull_blocks, sub_task).await;
            sync_blocks(blocks, self.pool.clone()).await?;
        }

        free_one_task();
        Ok(())
    }

    async fn pull_blocks(&self, numbers: Vec<u64>) -> Result<Vec<BlockView>> {
        let ret = self.adapter.pull_blocks(numbers).await?;
        Ok(ret)
    }

    pub async fn sync_indexer_cell_process(mut self) -> Result<()> {
        if self.type_.is_metadata_task() {
            return Err(anyhow!("{:?} Task type mismatch", self.type_));
        }

        add_one_task();

        if self.state_cursor.is_none() {
            self.set_state_cursor().await?;
        }

        let cursor = self.state_cursor.unwrap();
        let last = self.last_number();

        log::info!(
            "[sync] Sync indexer cell task {:?}, sync from {:?} to {:?}",
            self.id,
            cursor,
            last
        );

        for start in (cursor..=last).step_by(PULL_BLOCK_BATCH_SIZE as usize) {
            let end = (start + PULL_BLOCK_BATCH_SIZE).min(last + 1);
            let sub_task = (start..end).collect::<Vec<_>>();
            sync_indexer_cells(&sub_task, self.store.clone()).await?;
        }

        free_one_task();
        Ok(())
    }

    async fn poll_call<'a, A, B, F, Fut>(&'a self, f: F, input: A) -> B
    where
        A: Clone,
        B: Clone,
        F: Fn(&'a Task<T>, A) -> Fut,
        Fut: Future<Output = Result<B>>,
    {
        for _try in 0..10 {
            if let Ok(ret) = f(self, input.clone()).await {
                return ret;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        }

        panic!("Pulling blocks from node has failed 10 times");
    }

    fn last_number(&self) -> u64 {
        (self.id + TASK_LEN - 1).min(self.tip)
    }
}

async fn sync_blocks(blocks: Vec<BlockView>, pool: SQLXPool) -> Result<()> {
    let mut tx = pool.transaction().await?;

    bulk_insert_blocks(&blocks, &mut tx).await?;

    for block in blocks.iter() {
        let block_number = block.number();
        let block_hash = block.hash().raw_data().to_vec();
        let block_timestamp = block.timestamp();
        let epoch = block.epoch();
        let tx_views = block.transactions();

        bulk_insert_transactions(
            block_number,
            &block_hash,
            block_timestamp,
            &tx_views,
            &mut tx,
        )
        .await?;
        bulk_insert_output_cells(block_number, &block_hash, epoch, &tx_views, false, &mut tx)
            .await?;
        bulk_insert_consume_info(block_number, &block_hash, &tx_views, &mut tx).await?;
    }

    tx.commit().await.map_err(Into::into)
}

async fn sync_indexer_cells(sub_task: &[u64], rdb: XSQLPool) -> Result<()> {
    let mut indexer_cells = Vec::new();
    let mut tx = rdb.transaction().await?;
    let mut status_list = Vec::new();

    let w = rdb
        .wrapper()
        .r#in("block_number", sub_task)
        .or()
        .r#in("consumed_block_number", sub_task);
    let cells = tx.fetch_list_by_wrapper::<CellTable>(w).await?;

    for cell in cells.iter() {
        if sub_task.contains(&cell.block_number) {
            let i_cell = IndexerCellTable::new_with_empty_scripts(
                cell.block_number,
                IO_TYPE_OUTPUT,
                cell.output_index,
                cell.tx_hash.clone(),
                cell.tx_index,
            );
            indexer_cells.push(i_cell.update_by_cell_table(cell));
        }

        if let Some(consume_number) = cell.consumed_block_number {
            if sub_task.contains(&consume_number) {
                let i_cell = IndexerCellTable::new_with_empty_scripts(
                    consume_number,
                    IO_TYPE_INPUT,
                    cell.input_index.expect("cell input index"),
                    cell.consumed_tx_hash.clone(),
                    cell.consumed_tx_index.expect("cell consumed tx index"),
                );
                indexer_cells.push(i_cell.update_by_cell_table(cell));
            }
        }
    }

    status_list.extend(sub_task.iter().map(|num| SyncStatus::new(*num)));

    indexer_cells.sort();
    indexer_cells
        .iter_mut()
        .for_each(|c| c.id = generate_id(c.block_number));
    core_storage::save_batch_slice!(tx, indexer_cells, status_list);

    commit_transaction(tx).await?;

    Ok(())
}

async fn sync_indexer_cells_(sub_task: &[u64], pool: SQLXPool) -> Result<()> {
    todo!()
}

async fn bulk_insert_consume_info(
    block_number: u64,
    block_hash: &[u8],
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    let mut consume_info_rows = Vec::new();

    for (tx_index, transaction) in tx_views.iter().enumerate() {
        if tx_index == 0 {
            continue;
        }

        let tx_hash = transaction.hash().raw_data();

        for (input_index, input) in transaction.inputs().into_iter().enumerate() {
            let previous_output = input.previous_output();
            let previous_output_tx_hash = previous_output.tx_hash().raw_data();
            let previous_output_index: u32 = previous_output.index().unpack();
            let since: u64 = input.since().unpack();

            let consume_info = (
                previous_output_tx_hash.to_vec(),
                previous_output_index as i32,
                block_number as i64,
                block_hash.to_vec(),
                tx_hash.to_vec(),
                tx_index as i32,
                input_index as i32,
                since.to_be_bytes().to_vec(),
            );

            consume_info_rows.push(consume_info);
        }
    }

    // bulk insert
    for start in (0..consume_info_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(consume_info_rows.len());

        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_consume_info");
        builder.field(
            r#"
            tx_hash,
            output_index,
            consumed_block_number,
            consumed_block_hash,
            consumed_tx_hash,
            consumed_tx_index,
            input_index,
            since"#,
        );
        push_values_placeholders(&mut builder, 8, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in consume_info_rows.iter() {
            seq!(i in 0..8 {
                query = query.bind(&row.i);
            });
        }

        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}
