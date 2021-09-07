mod table;

use crate::table::SyncStatus;

use common::{async_trait, Result};
use core_storage::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, ConsumeInfoTable, TransactionTable,
    UncleRelationshipTable,
};
use core_storage::relational::{generate_id, to_bson_bytes};
use db_protocol::KVStore;
use db_rocksdb::RocksdbStore;
use db_xsql::{rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::prelude::*;
use tokio::time::sleep;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

const SYNC_TASK_BATCH_SIZE: usize = 10_000;
const PULL_BLOCK_BATCH_SIZE: usize = 10;
const CELL_TABLE_BATCH_SIZE: usize = 1_000;
const CONSUME_TABLE_BATCH_SIZE: usize = 2000;

macro_rules! save_batch {
	($tx: expr$ (, $table: expr)*) => {{
		$($tx.save_batch(&$table, &[]).await?;)*
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
    rocksdb: RocksdbStore,
    adapter: Arc<T>,
}

impl<T: SyncAdapter> Synchronization<T> {
    pub fn new(pool: XSQLPool, rocksdb_path: &str, adapter: Arc<T>) -> Self {
        let rocksdb = RocksdbStore::new(rocksdb_path);

        Synchronization {
            pool,
            rocksdb,
            adapter,
        }
    }

    pub async fn do_sync(&self, chain_tip: BlockNumber) -> Result<()> {
        let sync_list = self.build_to_sync_list(chain_tip).await?;
        let this = Arc::new(());

        for set in sync_list.chunks(SYNC_TASK_BATCH_SIZE) {
            let sync_set = set.to_vec();
            let (rdb, kvdb, adapter, arc_clone) = (
                self.pool.clone(),
                self.rocksdb.clone(),
                Arc::clone(&self.adapter),
                Arc::clone(&this),
            );

            tokio::spawn(async move {
                sync_process(sync_set, rdb, kvdb, adapter, arc_clone).await;
            });
        }

        while Arc::strong_count(&this) != 1 {
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn build_to_sync_list(&self, chain_tip: u64) -> Result<Vec<BlockNumber>> {
        let mut to_sync_number_set = (1..=chain_tip).collect::<HashSet<_>>();
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
}

async fn sync_process<T: SyncAdapter>(
    task: Vec<BlockNumber>,
    rdb: XSQLPool,
    kvdb: RocksdbStore,
    adapter: Arc<T>,
    _: Arc<()>,
) {
    for subtask in task.chunks(PULL_BLOCK_BATCH_SIZE) {
        log::info!("[sync] sync from {}", subtask[0]);

        let (rdb_clone, kvdb_clone, adapter_clone) =
            (rdb.clone(), kvdb.clone(), Arc::clone(&adapter));

        if let Err(err) = sync_blocks(subtask.to_vec(), rdb_clone, kvdb_clone, adapter_clone).await
        {
            panic!("[sync] sync error {:?}", err);
        }
    }
}

async fn sync_blocks<T: SyncAdapter>(
    task: Vec<BlockNumber>,
    rdb: XSQLPool,
    _kvdb: RocksdbStore,
    adapter: Arc<T>,
) -> Result<()> {
    let blocks = adapter
        .pull_blocks(task.clone())
        .await
        .unwrap_or_else(|e| panic!("pull blocks error {:?}, task {:?}", e, task));
    let mut block_table_batch: Vec<BlockTable> = Vec::new();
    let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
    let mut cell_table_batch: Vec<CellTable> = Vec::new();
    let mut consume_info_batch: Vec<ConsumeInfoTable> = Vec::new();
    let mut uncle_relationship_table_batch: Vec<UncleRelationshipTable> = Vec::new();
    let mut canonical_data_table_batch: Vec<CanonicalChainTable> = Vec::new();
    let mut sync_status_table_batch: Vec<SyncStatus> = Vec::new();
    let mut tx = rdb.transaction().await?;

    for block in blocks.iter() {
        let block_number = block.number();
        let block_hash = block.hash().raw_data().to_vec();
        let block_timestamp = block.timestamp();
        let block_epoch = block.epoch();

        block_table_batch.push(block.into());
        uncle_relationship_table_batch.push(UncleRelationshipTable::new(
            to_bson_bytes(&block_hash),
            to_bson_bytes(&block.uncle_hashes().as_bytes()),
        ));
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
                            uncle_relationship_table_batch,
                            canonical_data_table_batch,
                            sync_status_table_batch
                        );

                        clear_batch!(
                            block_table_batch,
                            tx_table_batch,
                            cell_table_batch,
                            consume_info_batch,
                            uncle_relationship_table_batch,
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
                        uncle_relationship_table_batch,
                        canonical_data_table_batch,
                        sync_status_table_batch
                    );

                    clear_batch!(
                        block_table_batch,
                        tx_table_batch,
                        cell_table_batch,
                        consume_info_batch,
                        uncle_relationship_table_batch,
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
        uncle_relationship_table_batch,
        canonical_data_table_batch,
        sync_status_table_batch
    );

    tx.commit().await?;

    Ok(())
}
