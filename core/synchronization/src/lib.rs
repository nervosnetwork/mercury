mod sql;
mod table;

use crate::table::{ConsumeInfoTable, InUpdate, SyncStatus};

use common::{async_trait, Result};
use core_storage::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, TransactionTable,
};
use core_storage::relational::{generate_id, to_bson_bytes};
use db_xsql::{rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::prelude::*;
use parking_lot::RwLock;
use rbatis::executor::RBatisTxExecutor;
use tokio::time::sleep;

use std::collections::HashSet;
use std::{sync::Arc, time::Duration};

const PULL_BLOCK_BATCH_SIZE: usize = 10;
const CELL_TABLE_BATCH_SIZE: usize = 1_000;
const CONSUME_TABLE_BATCH_SIZE: usize = 2_000;

lazy_static::lazy_static! {
    static ref CURRENT_TASK_NUMBER: RwLock<usize> = RwLock::new(0);
    static ref OUT_POINT_PREFIX: &'static [u8] = &b"\xFFout_point"[..];
    static ref IN_UPDATE_KEY: &'static [u8] = &b"in_update"[..];
}

macro_rules! save_batch {
	($tx: expr$ (, $table: expr)*) => {{
		$(if $tx.save_batch(&$table, &[]).await.is_err() {
            $tx.rollback().await?;
            return Ok(());
        })*
	}};
}

macro_rules! clear_batch {
    ($($table: expr), *) => {{
		$($table.clear();)*
	}};
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
        self.wait_insertion_complete().await;

        let current_count = {
            let w = self.pool.wrapper();
            self.pool.fetch_count_by_wrapper::<BlockTable>(&w).await?
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
        self.set_in_update().await?;
        let mut tx = self.pool.transaction().await.unwrap();
        sql::drop_live_cell_table(&mut tx).await.unwrap();
        sql::drop_script_table(&mut tx).await.unwrap();
        sql::create_live_cell_table(&mut tx).await.unwrap();
        sql::create_script_table(&mut tx).await.unwrap();
        sql::update_cell_table(&mut tx).await.unwrap();
        sql::insert_into_live_cell(&mut tx).await.unwrap();
        sql::insert_into_script(&mut tx).await.unwrap();
        sql::drop_consume_info_table(&mut tx).await.unwrap();
        self.remove_in_update(&mut tx).await.unwrap();
        tx.commit().await.expect("insert into");
        let _ = tx.take_conn().unwrap().close().await;

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
        let res = self.pool.fetch_list::<SyncStatus>().await?;
        Ok(res.iter().map(|t| t.block_number).collect())
    }

    async fn get_tip_number(&self) -> Result<BlockNumber> {
        let w = self
            .pool
            .wrapper()
            .order_by(false, &["block_number"])
            .limit(1);
        let res = self
            .pool
            .fetch_by_wrapper::<CanonicalChainTable>(&w)
            .await?;
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
        Ok(self.pool.fetch_count_by_wrapper::<InUpdate>(&w).await? == 1)
    }

    async fn set_in_update(&self) -> Result<()> {
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
        tx.remove_by_wrapper::<InUpdate>(&w).await?;
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
    let mut sync_status_table_batch: Vec<SyncStatus> = Vec::new();
    let mut tx = rdb.transaction().await?;

    for block in blocks.iter() {
        let block_number = block.number();
        let block_hash = block.hash().raw_data().to_vec();
        let block_timestamp = block.timestamp();
        let block_epoch = block.epoch();

        block_table_batch.push(block.into());
        canonical_data_table_batch.push(CanonicalChainTable::new(
            block_number,
            to_bson_bytes(&block_hash),
        ));
        sync_status_table_batch.push(SyncStatus::new(block_number));

        for (tx_idx, transaction) in block.transactions().iter().enumerate() {
            let tx_hash = to_bson_bytes(&transaction.hash().raw_data());
            tx_table_batch.push(TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                tx_idx as u32,
                to_bson_bytes(&block_hash),
                block_number,
                block_timestamp,
            ));

            // skip cellbase
            if tx_idx != 0 {
                for (i, input) in transaction.inputs().into_iter().enumerate() {
                    consume_info_batch.push(ConsumeInfoTable::new(
                        input.previous_output(),
                        block_number,
                        to_bson_bytes(&block_hash),
                        tx_hash.clone(),
                        tx_idx as u32,
                        i as u32,
                        input.since().unpack(),
                    ));

                    if consume_info_batch.len() > CONSUME_TABLE_BATCH_SIZE {
                        save_batch!(
                            tx,
                            block_table_batch,
                            tx_table_batch,
                            cell_table_batch,
                            consume_info_batch,
                            canonical_data_table_batch,
                            sync_status_table_batch
                        );

                        clear_batch!(
                            block_table_batch,
                            tx_table_batch,
                            cell_table_batch,
                            consume_info_batch,
                            canonical_data_table_batch,
                            sync_status_table_batch
                        );
                    }
                }
            }

            for (i, (cell, data)) in transaction.outputs_with_data_iter().enumerate() {
                let cell_table = CellTable::from_cell(
                    &cell,
                    generate_id(block_number),
                    tx_hash.clone(),
                    i as u32,
                    tx_idx as u32,
                    block_number,
                    to_bson_bytes(&block_hash),
                    block_epoch,
                    &data,
                );

                cell_table_batch.push(cell_table);

                if cell_table_batch.len() > CELL_TABLE_BATCH_SIZE {
                    save_batch!(
                        tx,
                        block_table_batch,
                        tx_table_batch,
                        cell_table_batch,
                        consume_info_batch,
                        canonical_data_table_batch,
                        sync_status_table_batch
                    );

                    clear_batch!(
                        block_table_batch,
                        tx_table_batch,
                        cell_table_batch,
                        consume_info_batch,
                        canonical_data_table_batch,
                        sync_status_table_batch
                    );
                }
            }
        }
    }

    save_batch!(
        tx,
        block_table_batch,
        tx_table_batch,
        cell_table_batch,
        consume_info_batch,
        canonical_data_table_batch,
        sync_status_table_batch
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
