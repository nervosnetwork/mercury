mod sql;
mod table;

use crate::table::{ConsumeInfoTable, InUpdate};

use common::{async_trait, Result};
use core_storage::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, IndexerCellTable, TransactionTable, IO_TYPE_INPUT,
    IO_TYPE_OUTPUT,
};
use core_storage::relational::{generate_id, to_rb_bytes, BATCH_SIZE_THRESHOLD};
use db_xsql::{rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::prelude::*;
use parking_lot::RwLock;
use rbatis::executor::RBatisTxExecutor;
use tokio::time::sleep;

use std::collections::HashSet;
use std::{ops::Range, sync::Arc, time::Duration};

const PULL_BLOCK_BATCH_SIZE: usize = 10;
const INSERT_INTO_BATCH_SIZE: usize = 200_000;
#[allow(dead_code)]
const INSERT_INDEXER_CELL_TABLE_SIZE: usize = 2_500;

lazy_static::lazy_static! {
    static ref CURRENT_TASK_NUMBER: RwLock<usize> = RwLock::new(0);
}

#[async_trait]
pub trait SyncAdapter: Sync + Send + 'static {
    /// Pull blocks by block number when synchronizing.
    async fn pull_blocks(&self, block_numbers: Vec<BlockNumber>) -> Result<Vec<BlockView>>;
}

pub struct Synchronization<T> {
    pool: XSQLPool,
    adapter: Arc<T>,

    sync_task_size: usize,
    max_task_number: usize,
}

impl<T: SyncAdapter> Synchronization<T> {
    pub fn new(
        pool: XSQLPool,
        adapter: Arc<T>,
        sync_task_size: usize,
        max_task_number: usize,
    ) -> Self {
        Synchronization {
            pool,
            adapter,
            sync_task_size,
            max_task_number,
        }
    }

    pub async fn do_sync(&self, chain_tip: BlockNumber) -> Result<()> {
        let sync_list = self.build_to_sync_list(chain_tip).await?;
        self.try_create_consume_info_table().await?;
        self.sync_batch_insert(chain_tip, sync_list).await;
        self.set_in_update().await?;
        self.wait_insertion_complete().await;

        let current_count = {
            let w = self.pool.wrapper();
            self.pool.fetch_count_by_wrapper::<BlockTable>(w).await?
        };

        log::info!("[sync] current block count {}", current_count);

        let mut num = 1;
        while let Some(set) = self.check_synchronization().await? {
            log::info!("[sync] resync {} time", num);
            self.sync_batch_insert(chain_tip, set).await;
            self.wait_insertion_complete().await;
            num += 1;
        }

        log::info!("[sync] insert into live cell table");
        let mut tx = self.pool.transaction().await.unwrap();
        sql::drop_live_cell_table(&mut tx).await.unwrap();
        sql::drop_script_table(&mut tx).await.unwrap();
        sql::create_live_cell_table(&mut tx).await.unwrap();
        sql::create_script_table(&mut tx).await.unwrap();

        for i in page_range(chain_tip, INSERT_INTO_BATCH_SIZE).step_by(INSERT_INTO_BATCH_SIZE) {
            let end = i + INSERT_INTO_BATCH_SIZE as u32;
            log::info!("[sync] update cell table from {} to {}", i, end);
            sql::update_cell_table(&mut tx, i, end).await.unwrap();
        }

        for i in page_range(chain_tip, INSERT_INTO_BATCH_SIZE).step_by(INSERT_INTO_BATCH_SIZE) {
            let end = i + INSERT_INTO_BATCH_SIZE as u32;
            log::info!("[sync] insert into live cell table {} to {}", i, end);
            sql::insert_into_live_cell(&mut tx, i, end).await.unwrap();
        }

        log::info!("[sync] insert into script table");
        sql::insert_into_script(&mut tx).await.unwrap();

        // log::info!("[sync] build indexer cell table");
        // self.build_indexer_cell_table(chain_tip, &mut tx)
        //     .await
        //     .unwrap();

        sql::drop_consume_info_table(&mut tx).await.unwrap();
        self.remove_in_update(&mut tx).await.unwrap();
        tx.commit().await.expect("insert into");
        let _ = tx.take_conn().unwrap().close().await;

        Ok(())
    }

    async fn _build_indexer_cell_table(
        &self,
        chain_tip: u64,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        // Todo: can do perf here. Use a Lock-free concurrent data structure
        // such as corssbeam::SegQueue instead of Vec.
        let mut indexer_cells = Vec::new();

        for i in page_range(chain_tip, INSERT_INDEXER_CELL_TABLE_SIZE)
            .step_by(INSERT_INDEXER_CELL_TABLE_SIZE)
        {
            let end = i + (INSERT_INDEXER_CELL_TABLE_SIZE as u32) - 1;
            let block_number_range = Range {
                start: i as u64,
                end: end as u64 + 1,
            };

            log::info!("[sync]build indexer table from {} to {}", i, end);
            let w = self
                .pool
                .wrapper()
                .between("block_number", i, end)
                .or()
                .between("consumed_block_number", i, end);
            let cells = tx.fetch_list_by_wrapper::<CellTable>(w).await?;

            for cell in cells.iter() {
                if block_number_range.contains(&cell.block_number) {
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
                    if block_number_range.contains(&consume_number) {
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

            indexer_cells.sort();
            indexer_cells
                .iter_mut()
                .for_each(|c| c.id = generate_id(c.block_number));
            core_storage::save_batch_slice!(tx, indexer_cells);
        }

        Ok(())
    }

    async fn try_create_consume_info_table(&self) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let _ = sql::create_consume_info_table(&mut conn).await;
        Ok(())
    }

    async fn sync_batch_insert(&self, chain_tip: u64, sync_list: Vec<u64>) {
        log::info!(
            "[sync] chain tip is {}, need sync {}",
            chain_tip,
            sync_list.len()
        );

        for set in sync_list.chunks(self.sync_task_size) {
            let sync_set = set.to_vec();
            let (rdb, adapter) = (self.pool.clone(), Arc::clone(&self.adapter));

            loop {
                let task_num = current_task_count();
                if task_num < self.max_task_number {
                    add_one_task();
                    tokio::spawn(async move {
                        sync_process(sync_set, rdb, adapter).await;
                    });

                    break;
                } else {
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn build_to_sync_list(&self, chain_tip: u64) -> Result<Vec<BlockNumber>> {
        let mut to_sync_number_set = (0..=chain_tip).collect::<HashSet<_>>();
        let sync_completed_set = self.get_sync_completed_numbers().await?;
        sync_completed_set.iter().for_each(|num| {
            to_sync_number_set.remove(num);
        });

        Ok(to_sync_number_set.into_iter().collect())
    }

    async fn get_sync_completed_numbers(&self) -> Result<Vec<BlockNumber>> {
        let mut conn = self.pool.acquire().await?;
        let res = sql::get_sync_completed_numbers(&mut conn).await?;
        Ok(res.iter().map(|t| t.block_number).collect())
    }

    async fn get_tip_number(&self) -> Result<BlockNumber> {
        let w = self
            .pool
            .wrapper()
            .order_by(false, &["block_number"])
            .limit(1);
        let res = self.pool.fetch_by_wrapper::<CanonicalChainTable>(w).await?;
        Ok(res.block_number)
    }

    async fn check_synchronization(&self) -> Result<Option<Vec<BlockNumber>>> {
        let tip_number = self.get_tip_number().await?;
        let set = self.build_to_sync_list(tip_number).await?;
        if set.is_empty() {
            Ok(None)
        } else {
            Ok(Some(set))
        }
    }

    async fn wait_insertion_complete(&self) {
        loop {
            sleep(Duration::from_secs(5)).await;

            let task_num = current_task_count();
            if task_num == 0 {
                return;
            }

            log::info!("current thread number {}", current_task_count());
        }
    }

    pub async fn is_previous_in_update(&self) -> Result<bool> {
        let w = self.pool.wrapper();
        Ok(self.pool.fetch_count_by_wrapper::<InUpdate>(w).await? == 1)
    }

    async fn set_in_update(&self) -> Result<()> {
        if self.is_previous_in_update().await? {
            return Ok(());
        }

        let mut acquire = self.pool.acquire().await?;
        acquire.save(&InUpdate { is_in: true }, &[]).await?;
        Ok(())
    }

    async fn remove_in_update(&self, tx: &mut RBatisTxExecutor<'_>) -> Result<()> {
        let w = self
            .pool
            .wrapper()
            .eq("is_in", true)
            .or()
            .eq("is_in", false);
        tx.remove_by_wrapper::<InUpdate>(w).await?;
        Ok(())
    }
}

async fn sync_process<T: SyncAdapter>(task: Vec<BlockNumber>, rdb: XSQLPool, adapter: Arc<T>) {
    for subtask in task.chunks(PULL_BLOCK_BATCH_SIZE) {
        let (rdb_clone, adapter_clone) = (rdb.clone(), Arc::clone(&adapter));

        if let Err(err) = sync_blocks(subtask.to_vec(), rdb_clone, adapter_clone).await {
            log::error!("[sync] sync block {:?} error {:?}", subtask, err)
        }
    }

    free_one_task();
}

async fn sync_blocks<T: SyncAdapter>(
    task: Vec<BlockNumber>,
    rdb: XSQLPool,
    adapter: Arc<T>,
) -> Result<()> {
    let blocks = adapter.pull_blocks(task.clone()).await?;
    let mut block_table_batch: Vec<BlockTable> = Vec::new();
    let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
    let mut cell_table_batch: Vec<CellTable> = Vec::new();
    let mut consume_info_batch: Vec<ConsumeInfoTable> = Vec::new();
    let mut canonical_data_table_batch: Vec<CanonicalChainTable> = Vec::new();
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

    tx.commit().await?;

    let _ = tx.take_conn().unwrap().close().await;

    Ok(())
}

fn current_task_count() -> usize {
    *CURRENT_TASK_NUMBER.read()
}

fn add_one_task() {
    let mut num = CURRENT_TASK_NUMBER.write();
    *num += 1;
}

fn free_one_task() {
    let mut num = CURRENT_TASK_NUMBER.write();
    *num -= 1;
}

fn page_range(chain_tip: u64, step_len: usize) -> Range<u32> {
    let count = chain_tip / step_len as u64 + 1;
    Range {
        start: 0u32,
        end: (count as u32) * (step_len as u32) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let range = page_range(1_000_000, INSERT_INTO_BATCH_SIZE);
        for i in range.step_by(INSERT_INTO_BATCH_SIZE) {
            let end = i + INSERT_INTO_BATCH_SIZE as u32;
            println!("start {} end {}", i, end);
        }
    }
}
