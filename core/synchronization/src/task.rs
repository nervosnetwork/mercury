use crate::table::ConsumeInfoTable;
use crate::{add_one_task, free_one_task, SyncAdapter};

use common::{anyhow::anyhow, Result};
use core_storage::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, IndexerCellTable, SyncStatus, TransactionTable,
    IO_TYPE_INPUT, IO_TYPE_OUTPUT,
};
use core_storage::relational::{generate_id, to_rb_bytes, BATCH_SIZE_THRESHOLD};
use db_xsql::{commit_transaction, rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::{core::BlockView, prelude::*};
use tokio::time::sleep;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

pub const TASK_LEN: u64 = 100_000;
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
    store: XSQLPool,
    type_: TaskType,
    state_cursor: Option<u64>,

    adapter: Arc<T>,
}

impl<T: SyncAdapter> Task<T> {
    pub fn new(id: u64, store: XSQLPool, adapter: Arc<T>, type_: TaskType) -> Task<T> {
        Task {
            id,
            store,
            type_,
            state_cursor: None,
            adapter,
        }
    }

    async fn set_state_cursor(&mut self) -> Result<()> {
        let last = self.last_number();

        let w = self
            .store
            .wrapper()
            .between("block_number", self.id, last)
            .order_by(false, &["block_number"])
            .limit(1);

        let cursor = if self.type_.is_metadata_task() {
            let block: Option<BlockTable> = self.store.fetch_by_wrapper(w).await?;
            block.map_or_else(|| self.id, |b| (b.block_number + 1).min(last))
        } else {
            let cell: Option<IndexerCellTable> = self.store.fetch_by_wrapper(w).await?;
            cell.map_or_else(|| self.id, |c| (c.block_number + 1).min(last))
        };

        self.state_cursor = Some(cursor);
        Ok(())
    }

    pub async fn is_done(&mut self) -> Result<bool> {
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
            sync_blocks(blocks, self.store.clone()).await?;
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
        self.id + TASK_LEN - 1
    }
}

async fn sync_blocks(blocks: Vec<BlockView>, rdb: XSQLPool) -> Result<()> {
    let mut block_table_batch: Vec<BlockTable> = Vec::new();
    let mut tx_table_batch = Vec::new();
    let mut cell_table_batch = Vec::new();
    let mut consume_info_batch = Vec::new();
    let mut canonical_data_table_batch = Vec::new();
    let mut tx = rdb.transaction().await?;

    for block in blocks.iter() {
        let block_number = block.number();
        let block_hash = block.hash().raw_data().to_vec();
        let block_timestamp = block.timestamp();
        let block_epoch = block.epoch();

        block_table_batch.push(block.into());
        canonical_data_table_batch.push(CanonicalChainTable::new(
            block_number,
            to_rb_bytes(&block_hash),
        ));

        for (tx_idx, transaction) in block.transactions().iter().enumerate() {
            let tx_hash = to_rb_bytes(&transaction.hash().raw_data());
            tx_table_batch.push(TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                tx_idx as u32,
                to_rb_bytes(&block_hash),
                block_number,
                block_timestamp,
            ));

            // skip cellbase
            if tx_idx != 0 {
                for (input_idx, input) in transaction.inputs().into_iter().enumerate() {
                    consume_info_batch.push(ConsumeInfoTable::new(
                        input.previous_output(),
                        block_number,
                        to_rb_bytes(&block_hash),
                        tx_hash.clone(),
                        tx_idx as u32,
                        input_idx as u32,
                        input.since().unpack(),
                    ));
                }
            }

            for (output_idx, (cell, data)) in transaction.outputs_with_data_iter().enumerate() {
                cell_table_batch.push(CellTable::from_cell(
                    &cell,
                    generate_id(block_number),
                    tx_hash.clone(),
                    output_idx as u32,
                    tx_idx as u32,
                    block_number,
                    to_rb_bytes(&block_hash),
                    block_epoch,
                    &data,
                ));
            }
        }
    }

    core_storage::save_batch_slice!(
        tx,
        block_table_batch,
        tx_table_batch,
        cell_table_batch,
        consume_info_batch,
        canonical_data_table_batch
    );

    commit_transaction(tx).await?;

    Ok(())
}

pub async fn sync_indexer_cells(sub_task: &[u64], rdb: XSQLPool) -> Result<()> {
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
                    cell.input_index.unwrap(),
                    cell.consumed_tx_hash.clone(),
                    cell.consumed_tx_index.unwrap(),
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
    core_storage::save_batch_slice!(tx, indexer_cells);

    commit_transaction(tx).await?;

    Ok(())
}
