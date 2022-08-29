use crate::{add_one_task, free_one_task, SyncAdapter, TASK_LEN};

use common::{anyhow::anyhow, Result};
use core_storage::relational::{
    bulk_insert_blocks, bulk_insert_output_cells, bulk_insert_transactions, generate_id,
    push_values_placeholders, BATCH_SIZE_THRESHOLD, IO_TYPE_INPUT, IO_TYPE_OUTPUT,
};
use db_sqlx::SQLXPool;

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
    pool: SQLXPool,
    type_: TaskType,
    state_cursor: Option<u64>,

    adapter: Arc<T>,
}

impl<T: SyncAdapter> Task<T> {
    pub fn new(id: u64, tip: u64, pool: SQLXPool, adapter: Arc<T>, type_: TaskType) -> Task<T> {
        Task {
            id,
            tip,
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
            |row| (row.get::<i32, _>("block_number") as u64 + 1).min(last),
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
            sync_indexer_cells(&sub_task, self.pool.clone()).await?;
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

async fn sync_indexer_cells(sub_task: &[u64], pool: SQLXPool) -> Result<()> {
    let mut tx = pool.transaction().await?;
    bulk_insert_indexer_cells(sub_task, &mut tx).await?;
    bulk_insert_sync_status(sub_task, &mut tx).await?;
    tx.commit().await.map_err(Into::into)
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
        for row in consume_info_rows[start..end].iter() {
            seq!(i in 0..8 {
                query = query.bind(&row.i);
            });
        }

        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}

async fn bulk_insert_indexer_cells(sub_task: &[u64], tx: &mut Transaction<'_, Any>) -> Result<()> {
    let mut query = SqlBuilder::select_from("mercury_cell");
    query
        .field(
            "id, tx_hash, output_index, tx_index, block_number, 
            lock_hash, lock_code_hash, lock_args, lock_script_type,
            type_hash, type_code_hash, type_args, type_script_type,
            consumed_block_number, consumed_tx_hash, consumed_tx_index, input_index",
        )
        .and_where_in("block_number", sub_task)
        .or_where_in("consumed_block_number", sub_task);
    let sql = query.sql()?;
    let query = SQLXPool::new_query(&sql);
    let cells = query.fetch_all(&mut *tx).await?;

    let mut indexer_cell_rows = Vec::new();
    for cell in cells.iter() {
        if sub_task.contains(&(cell.get::<i32, _>("block_number") as u64)) {
            let indexer_cell = (
                0i64,
                cell.get::<i32, _>("block_number"),
                i16::try_from(IO_TYPE_OUTPUT)?,
                cell.get::<i32, _>("output_index"),
                cell.get::<Vec<u8>, _>("tx_hash"),
                cell.get::<i32, _>("tx_index"),
                cell.get::<Vec<u8>, _>("lock_hash"),
                cell.get::<Vec<u8>, _>("lock_code_hash"),
                cell.get::<Vec<u8>, _>("lock_args"),
                cell.get::<i16, _>("lock_script_type"),
                cell.get::<Vec<u8>, _>("type_hash"),
                cell.get::<Vec<u8>, _>("type_code_hash"),
                cell.get::<Vec<u8>, _>("type_args"),
                cell.get::<i16, _>("type_script_type"),
            );
            indexer_cell_rows.push(indexer_cell);
        }

        if let Some(consume_number) = cell.get::<Option<i64>, _>("consumed_block_number") {
            if sub_task.contains(&(consume_number as u64)) {
                let indexer_cell = (
                    0i64,
                    i32::try_from(consume_number)?,
                    i16::try_from(IO_TYPE_INPUT)?,
                    cell.get("input_index"),
                    cell.get("consumed_tx_hash"),
                    cell.get("consumed_tx_index"),
                    cell.get("lock_hash"),
                    cell.get("lock_code_hash"),
                    cell.get("lock_args"),
                    cell.get("lock_script_type"),
                    cell.get("type_hash"),
                    cell.get("type_code_hash"),
                    cell.get("type_args"),
                    cell.get("type_script_type"),
                );
                indexer_cell_rows.push(indexer_cell);
            }
        }
    }

    indexer_cell_rows.sort_unstable_by(|a, b| {
        if a.1 != b.1 {
            // 1 block_number
            a.1.cmp(&b.1)
        } else if a.5 != b.5 {
            // 5 tx_index
            a.5.cmp(&b.5)
        } else if a.2 != b.2 {
            // 2 io_type
            a.2.cmp(&b.2)
        } else {
            // 3 io_index
            a.3.cmp(&b.3)
        }
    });
    indexer_cell_rows
        .iter_mut()
        .for_each(|row| row.0 = generate_id(row.1 as u64));

    // bulk insert indexer cells
    for start in (0..indexer_cell_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(indexer_cell_rows.len());

        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_indexer_cell");
        builder.field(
            r#"id,
            block_number,
            io_type,
            io_index,
            tx_hash,
            tx_index,
            lock_hash,
            lock_code_hash,
            lock_args,
            lock_script_type,
            type_hash,
            type_code_hash,
            type_args,
            type_script_type"#,
        );
        push_values_placeholders(&mut builder, 14, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in indexer_cell_rows[start..end].iter() {
            seq!(i in 0..14 {
                query = query.bind(&row.i);
            });
        }
        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}

async fn bulk_insert_sync_status(sub_task: &[u64], tx: &mut Transaction<'_, Any>) -> Result<()> {
    let sync_status_rows: Vec<i32> = sub_task.iter().map(|num| *num as i32).collect();

    // bulk insert sync status
    for start in (0..sync_status_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(sync_status_rows.len());

        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_sync_status");
        builder.field("block_number");
        push_values_placeholders(&mut builder, 1, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in sync_status_rows[start..end].iter() {
            query = query.bind(row);
        }

        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}
