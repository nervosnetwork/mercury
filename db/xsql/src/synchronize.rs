use crate::table::{
    BlockTable, CanonicalChainTable, CellTable, LiveCellTable, ScriptTable, SyncDeadCell,
    SyncStatus, TransactionTable, UncleRelationshipTable,
};
use crate::{generate_id, sql, to_bson_bytes, BsonBytes, DBAdapter};

use common::anyhow::Result;

use ckb_types::{core::BlockView, packed, prelude::*};
use futures::stream::StreamExt;
use rbatis::crud::{CRUDMut, CRUD};
use rbatis::executor::RBatisTxExecutor;
use rbatis::{rbatis::Rbatis, wrapper::Wrapper};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashSet;
use std::sync::Arc;

macro_rules! save_list {
	($tx: expr$ (, $table_list: expr)*) => {{
		$($tx.save_batch(&$table_list, &[]).await?;)*
		$tx.commit().await?;
	}};
}

pub async fn sync_blocks_process<T: DBAdapter>(
    rb: Arc<Rbatis>,
    adapter: Arc<dyn DBAdapter>,
    range: (u64, u64),
    outpoint_tx: UnboundedSender<packed::OutPoint>,
    batch_size: usize,
) -> Result<()> {
    for numbers in (range.0..=range.1)
        .collect::<Vec<_>>()
        .chunks(batch_size)
        .into_iter()
    {
        let mut tx = rb.acquire_begin().await?;
        let num_set = build_need_sync_block_numbers(&mut tx, numbers.to_vec()).await?;
        let blocks = adapter.pull_blocks(num_set).await?;
        let mut block_table_batch: Vec<BlockTable> = Vec::new();
        let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
        let mut cell_table_batch: Vec<CellTable> = Vec::new();
        let mut script_table_batch: HashSet<ScriptTable> = HashSet::new();
        let mut uncle_relationship_table_batch: Vec<UncleRelationshipTable> = Vec::new();
        let mut canonical_data_table_batch: Vec<CanonicalChainTable> = Vec::new();
        let mut sync_status_table_batch: Vec<SyncStatus> = Vec::new();
        let mut huge_blocks = Vec::new();

        for block in blocks.iter() {
            if is_huge(block) {
                huge_blocks.push(block.clone());
                continue;
            }

            let block_number = block.number();
            let block_hash = block.hash().raw_data().to_vec();
            let block_timestamp = block.timestamp();
            let block_epoch = block.epoch();

            block_table_batch.push(block.into());
            uncle_relationship_table_batch.push(UncleRelationshipTable {
                block_hash: to_bson_bytes(&block_hash),
                uncle_hashes: to_bson_bytes(&block.uncle_hashes().as_bytes()),
            });
            canonical_data_table_batch.push(CanonicalChainTable {
                block_number,
                block_hash: to_bson_bytes(&block_hash),
            });
            sync_status_table_batch.push(SyncStatus::new(block_number as u32));

            for (idx, tx) in block.transactions().iter().enumerate() {
                let tx_hash = to_bson_bytes(&tx.hash().raw_data());
                tx_table_batch.push(TransactionTable::from_view(
                    tx,
                    generate_id(block_number),
                    idx as u32,
                    to_bson_bytes(&block_hash),
                    block_number,
                    block_timestamp,
                ));

                for (i, (cell, data)) in tx.outputs_with_data_iter().enumerate() {
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

                    script_table_batch
                        .insert(cell_table.to_lock_script_table(generate_id(block_number)));

                    if cell_table.has_type_script() {
                        script_table_batch
                            .insert(cell_table.to_type_script_table(generate_id(block_number)));
                    }

                    cell_table_batch.push(cell_table);
                }

                if idx != 0 {
                    tx.inputs()
                        .into_iter()
                        .for_each(|input| outpoint_tx.send(input.previous_output()).unwrap());
                }
            }
        }

        let live_cell_table_batch = cell_table_batch
            .clone()
            .into_iter()
            .map(Into::into)
            .collect::<Vec<LiveCellTable>>();
        let script_table_batch = script_table_batch.into_iter().collect::<Vec<_>>();

        save_list!(
            tx,
            block_table_batch,
            tx_table_batch,
            cell_table_batch,
            live_cell_table_batch,
            script_table_batch,
            uncle_relationship_table_batch,
            canonical_data_table_batch,
            sync_status_table_batch
        );

        handle_huge_blocks(Arc::clone(&rb), huge_blocks.clone(), outpoint_tx.clone()).await?;
        huge_blocks.clear();
    }

    Ok(())
}

#[allow(clippy::collapsible_else_if)]
pub async fn handle_out_point(
    rb: Arc<Rbatis>,
    rx: UnboundedReceiver<packed::OutPoint>,
    success_tx: UnboundedSender<()>,
) -> Result<()> {
    let mut stream = UnboundedReceiverStream::new(rx);
    let wrapper = rb.new_wrapper_table::<LiveCellTable>();

    while let Some(out_point) = stream.next().await {
        let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
        let index: u32 = out_point.index().unpack();
        let w = build_wrapper(&wrapper, tx_hash.clone(), index);
        let mut tx = rb.acquire_begin().await?;
        let table = SyncDeadCell::new(tx_hash.clone(), index, false);

        let is_in_dead_cell_table = tx
            .fetch_by_wrapper::<Option<SyncDeadCell>>(&w)
            .await?
            .is_some();
        let is_remove = try_remove_live_cell(&mut tx, &w).await?;

        if is_in_dead_cell_table {
            if is_remove {
                sql::update_sync_dead_cell(&mut tx, tx_hash, index).await?;
            }
        } else {
            if is_remove {
                tx.save(&table.set_is_delete(), &[]).await?;
            } else {
                tx.save(&table, &[]).await?;
            }
        }

        tx.commit().await?;
    }

    let fetch_w = wrapper.clone().eq("is_delete", false).limit(100);
    let fetch_count_w = wrapper.clone().eq("is_delete", false);

    if rb
        .fetch_count_by_wrapper::<SyncDeadCell>(&fetch_count_w)
        .await?
        != 0
    {
        let mut tx = rb.acquire_begin().await?;
        for cell in tx
            .fetch_list_by_wrapper::<SyncDeadCell>(&fetch_w)
            .await?
            .into_iter()
        {
            log::info!("start clean dead cell");
            let w = build_wrapper(&wrapper, cell.tx_hash.clone(), cell.output_index);
            try_remove_live_cell(&mut tx, &w).await?;

            let table = cell.clone();
            tx.update_by_column("is_delete", &mut table.set_is_delete())
                .await?;
        }

        tx.commit().await?;
    }

    success_tx.send(()).unwrap();

    Ok(())
}

fn build_wrapper(wrapper: &Wrapper, tx_hash: BsonBytes, output_index: u32) -> Wrapper {
    let w = wrapper.clone();
    w.eq("tx_hash", tx_hash)
        .and()
        .eq("output_index", output_index)
}

async fn try_remove_live_cell(tx: &mut RBatisTxExecutor<'_>, wrapper: &Wrapper) -> Result<bool> {
    let ra = tx.remove_by_wrapper::<LiveCellTable>(wrapper).await?;
    Ok(ra == 1)
}

async fn build_need_sync_block_numbers(
    tx: &mut RBatisTxExecutor<'_>,
    input: Vec<u64>,
) -> Result<Vec<u64>> {
    let mut ret = Vec::new();
    for i in input.iter() {
        if tx
            .fetch_by_column::<Option<SyncStatus>, u32>("block_number", &(*i as u32))
            .await?
            .is_none()
        {
            ret.push(*i);
        }
    }

    Ok(ret)
}

fn is_huge(block: &BlockView) -> bool {
    let size = block
        .transactions()
        .iter()
        .map(|tx| tx.outputs().len())
        .sum::<usize>();
    size > 400
}

async fn handle_huge_blocks(
    rb: Arc<Rbatis>,
    blocks: Vec<BlockView>,
    out_point_tx: UnboundedSender<packed::OutPoint>,
) -> Result<()> {
    for block in blocks.into_iter() {
        let mut tx = rb.acquire_begin().await?;

        let block_number = block.number();
        let block_hash = block.hash().raw_data().to_vec();
        let block_timestamp = block.timestamp();
        let block_epoch = block.epoch();

        let block_table = BlockTable::from(&block);
        tx.save(&block_table, &[]).await?;

        let uncle_relationship_table = UncleRelationshipTable {
            block_hash: to_bson_bytes(&block_hash),
            uncle_hashes: to_bson_bytes(&block.uncle_hashes().as_bytes()),
        };
        tx.save(&uncle_relationship_table, &[]).await?;

        let canonical_data_table = CanonicalChainTable {
            block_number,
            block_hash: to_bson_bytes(&block_hash),
        };
        tx.save(&canonical_data_table, &[]).await?;

        let sync_state_table = SyncStatus::new(block_number as u32);
        tx.save(&sync_state_table, &[]).await?;

        for (idx, transaction) in block.transactions().iter().enumerate() {
            let tx_hash = to_bson_bytes(&transaction.hash().raw_data());
            let tx_table = TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                idx as u32,
                to_bson_bytes(&block_hash),
                block_number,
                block_timestamp,
            );
            tx.save(&tx_table, &[]).await?;

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

                tx.save(&cell_table, &[]).await?;

                let script_table = cell_table.to_lock_script_table(generate_id(block_number));
                tx.save(&script_table, &[]).await?;

                if cell_table.has_type_script() {
                    tx.save(
                        &cell_table.to_type_script_table(generate_id(block_number)),
                        &[],
                    )
                    .await?;
                }
            }

            if idx != 0 {
                transaction
                    .inputs()
                    .into_iter()
                    .for_each(|input| out_point_tx.send(input.previous_output()).unwrap());
            }
        }

        tx.commit().await?;
    }

    Ok(())
}

// struct InnerOutPoint {
//     tx_hash: Vec<u8>,
//     index: u32,
// }

// impl InnerOutPoint {
//     pub fn new(tx_hash: Vec<u8>, index: u32) -> Self {
//         InnerOutPoint { tx_hash, index }
//     }
// }
