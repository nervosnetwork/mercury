mod sql;
mod table;

use crate::table::SyncStatus;

use common::{async_trait, Result};
use core_storage::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, TransactionTable, UncleRelationshipTable,
};
use core_storage::relational::{generate_id, to_bson_bytes};
use db_protocol::{DBDriver, KVStore};
use db_rocksdb::RocksdbStore;
use db_xsql::{rbatis::crud::CRUDMut, XSQLPool};

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::prelude::*;
use log::LevelFilter;

use std::collections::HashSet;
use std::sync::Arc;

const SYNC_TASK_BATCH_SIZE: usize = 10_000;
const PULL_BLOCK_BATCH_SIZE: usize = 10;
const CELL_TABLE_BATCH_SIZE: usize = 1_000;

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
    pub fn new(
        max_connections: u32,
        center_id: u16,
        machine_id: u16,
        rocksdb_path: &str,
        adapter: Arc<T>,
    ) -> Self {
        let pool = XSQLPool::new(max_connections, center_id, machine_id, LevelFilter::Warn);
        let rocksdb = RocksdbStore::new(rocksdb_path);

        Synchronization {
            pool,
            rocksdb,
            adapter,
        }
    }

    pub async fn init(
        &self,
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.pool
            .connect(db_driver, db_name, host, port, user, password)
            .await?;
        Ok(())
    }

    pub async fn do_sync(&self, chain_tip: BlockNumber) -> Result<()> {
        let sync_list = self.build_to_sync_list(chain_tip).await?;

        for set in sync_list.chunks(SYNC_TASK_BATCH_SIZE) {
            let sync_set = set.to_vec();
            let (rdb, kvdb, adapter) = (
                self.pool.clone(),
                self.rocksdb.clone(),
                Arc::clone(&self.adapter),
            );

            tokio::spawn(async move {
                sync_process(sync_set, rdb, kvdb, adapter).await;
            });
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
) {
    for subtask in task.chunks(PULL_BLOCK_BATCH_SIZE) {
        let (rdb_clone, kvdb_clone, adapter_clone) = (rdb.clone(), kvdb.clone(), adapter.clone());
        
        if let Err(err) = sync_blocks(subtask.to_vec(), rdb_clone, kvdb_clone, adapter_clone).await
        {
            log::error!("[sync] sync error {:?}", err);
        }
    }
}

async fn sync_blocks<T: SyncAdapter>(
    task: Vec<BlockNumber>,
    rdb: XSQLPool,
    _kvdb: RocksdbStore,
    adapter: Arc<T>,
) -> Result<()> {
    let blocks = adapter.pull_blocks(task).await?;
    let mut block_table_batch: Vec<BlockTable> = Vec::new();
    let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
    let mut cell_table_batch: Vec<CellTable> = Vec::new();
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

        for (idx, transaction) in block.transactions().iter().enumerate() {
            let tx_hash = to_bson_bytes(&transaction.hash().raw_data());
            tx_table_batch.push(TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                idx as u32,
                to_bson_bytes(&block_hash),
                block_number,
                block_timestamp,
            ));

            for (i, (cell, data)) in transaction.outputs_with_data_iter().enumerate() {
                let cell_table = CellTable::from_cell(
                    &cell,
                    generate_id(block_number),
                    tx_hash.clone(),
                    i as u32,
                    idx as u32,
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
                        uncle_relationship_table_batch,
                        canonical_data_table_batch,
                        sync_status_table_batch
                    );

                    clear_batch!(
                        block_table_batch,
                        tx_table_batch,
                        cell_table_batch,
                        uncle_relationship_table_batch,
                        canonical_data_table_batch,
                        sync_status_table_batch
                    );
                }
            }
        }
    }

    tx.commit().await?;

    Ok(())
}
