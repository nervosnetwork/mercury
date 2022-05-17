mod sql;
mod table;
mod task;

use crate::table::InUpdate;
use crate::task::{Task, TaskType};

use common::{async_trait, Result};
use core_rpc_types::{SyncProgress, SyncState};
use db_xsql::{rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView};
use parking_lot::RwLock;
use rbatis::executor::RBatisTxExecutor;
use tokio::time::sleep;

use std::{ops::Range, sync::Arc, time::Duration};

pub const TASK_LEN: u64 = 100_000;
const INSERT_INTO_BATCH_SIZE: usize = 200_000;

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
    max_task_number: usize,
    chain_tip: u64,
    sync_state: Arc<RwLock<SyncState>>,

    adapter: Arc<T>,
}

impl<T: SyncAdapter> Synchronization<T> {
    pub fn new(
        pool: XSQLPool,
        adapter: Arc<T>,
        max_task_number: usize,
        chain_tip: u64,
        sync_state: Arc<RwLock<SyncState>>,
    ) -> Self {
        Synchronization {
            pool,
            max_task_number,
            chain_tip,
            sync_state,
            adapter,
        }
    }

    pub async fn do_sync(&self) -> Result<()> {
        if let Some(mut state) = self.sync_state.try_write() {
            *state = SyncState::ParallelFirstStage(SyncProgress::new(
                0,
                self.chain_tip,
                String::from("0.0%"),
            ));
            log::info!("[sync state] ParallelFirstStage");
        }

        self.try_create_consume_info_table().await?;
        self.sync_metadata().await;
        self.set_in_update().await?;
        self.wait_insertion_complete().await;

        log::info!("[sync] insert into live cell table");
        let mut tx = self.pool.transaction().await?;
        sql::drop_live_cell_table(&mut tx).await?;
        sql::drop_script_table(&mut tx).await?;
        sql::create_live_cell_table(&mut tx).await?;
        sql::create_script_table(&mut tx).await?;

        for i in page_range(self.chain_tip, INSERT_INTO_BATCH_SIZE).step_by(INSERT_INTO_BATCH_SIZE)
        {
            let end = i + INSERT_INTO_BATCH_SIZE as u32;
            log::info!("[sync] update cell table from {} to {}", i, end);
            sql::update_cell_table(&mut tx, &i, &end).await?
        }

        for i in page_range(self.chain_tip, INSERT_INTO_BATCH_SIZE).step_by(INSERT_INTO_BATCH_SIZE)
        {
            let end = i + INSERT_INTO_BATCH_SIZE as u32;
            log::info!("[sync] insert into live cell table {} to {}", i, end);
            sql::insert_into_live_cell(&mut tx, &i, &end).await?
        }

        log::info!("[sync] insert into script table");

        sql::insert_into_script(&mut tx).await?;
        sql::drop_consume_info_table(&mut tx).await?;

        log::info!("[sync] remove in update");

        self.remove_in_update(&mut tx).await?;
        tx.commit().await.expect("insert into");
        let _ = tx.take_conn().expect("take connection").close().await;
        sleep(Duration::from_secs(10)).await;
        Ok(())
    }

    pub async fn build_indexer_cell_table(&self) -> Result<()> {
        if let Some(mut state) = self.sync_state.try_write() {
            *state = SyncState::ParallelSecondStage(SyncProgress::new(0, 0, String::from("0.0%")));
            log::info!("[sync state] ParallelSecondStage");
        }

        for id in (0..=self.chain_tip).step_by(TASK_LEN as usize) {
            let mut task = Task::new(
                id,
                self.chain_tip,
                self.pool.clone(),
                Arc::clone(&self.adapter),
                TaskType::SyncIndexerCell,
            );

            if task.is_done().await? {
                continue;
            }

            loop {
                let task_number = current_task_count();
                if task_number < self.max_task_number {
                    tokio::spawn(async move {
                        let _ = task.sync_indexer_cell_process().await;
                    });
                    break;
                } else {
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }

        self.wait_insertion_complete().await;
        log::info!("[sync]finish");
        Ok(())
    }
    async fn try_create_consume_info_table(&self) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let _ = sql::create_consume_info_table(&mut conn).await;
        Ok(())
    }

    async fn sync_metadata(&self) {
        log::info!("[sync] chain tip is {}", self.chain_tip);

        for id in (0..=self.chain_tip).step_by(TASK_LEN as usize) {
            let mut task = Task::new(
                id,
                self.chain_tip,
                self.pool.clone(),
                Arc::clone(&self.adapter),
                TaskType::SyncMetadata,
            );

            if task.is_done().await.expect("task is done") {
                continue;
            }

            loop {
                let task_number = current_task_count();
                if task_number < self.max_task_number {
                    tokio::spawn(async move {
                        let _ = task.sync_metadata_process().await;
                    });
                    break;
                } else {
                    sleep(Duration::from_secs(5)).await;
                }
            }
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
        let w = self.pool.wrapper().eq("is_in", true);
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

fn current_task_count() -> usize {
    *CURRENT_TASK_NUMBER.read()
}

pub fn add_one_task() {
    let mut num = CURRENT_TASK_NUMBER.write();
    *num += 1;
}

pub fn free_one_task() {
    let mut num = CURRENT_TASK_NUMBER.write();
    *num -= 1;
}

fn page_range(chain_tip: u64, step_len: usize) -> Range<u32> {
    let count = chain_tip / step_len as u64 + 1;
    Range {
        start: 0u32,
        end: (count as u32) * (step_len as u32),
    }
}
