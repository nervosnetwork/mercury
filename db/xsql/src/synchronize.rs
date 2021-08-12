use crate::table::{
    BigDataTable, BlockTable, CellTable, LiveCellTable, ScriptTable, TransactionTable,
    UncleRelationshipTable,
};
use crate::{insert::BIG_DATA_THRESHOLD, sql, DBAdapter};

use common::anyhow::Result;

use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{packed, prelude::*};
use futures::stream::StreamExt;
use rbatis::crud::CRUDMut;
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashSet;

const INSERT_BATCH_SIZE: usize = 20;
const MAX_OUT_POINT_QUEUE_SIZE: usize = 5000;

macro_rules! save_list {
	($tx: expr$ (, $table_list: expr)*) => {{
		$($tx.save_batch(&$table_list, &[]).await?;)*
		$tx.commit().await?;
	}};
}

pub async fn sync_blocks_process<T: DBAdapter>(
    mut tx: RBatisTxExecutor<'_>,
    block_list: Vec<BlockView>,
    outpoint_tx: UnboundedSender<packed::OutPoint>,
    number_tx: UnboundedSender<u64>,
) -> Result<()> {
    let mut max_number = BlockNumber::MIN;
    for blocks in block_list.chunks(INSERT_BATCH_SIZE).into_iter() {
        let mut block_table_batch: Vec<BlockTable> = Vec::new();
        let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
        let mut cell_table_batch: Vec<CellTable> = Vec::new();
        let mut script_table_batch: HashSet<ScriptTable> = HashSet::new();
        let mut big_data_table_batch: Vec<BigDataTable> = Vec::new();
        let mut uncle_relationship_table_batch: Vec<UncleRelationshipTable> = Vec::new();

        for block in blocks.iter() {
            let block_number = block.number();
            let block_hash = block.hash().raw_data().to_vec();
            let block_timestamp = block.timestamp();
            let block_epoch = block.epoch().full_value();
            max_number = max_number.max(block_number);

            block_table_batch.push(block.into());
            uncle_relationship_table_batch.push(UncleRelationshipTable {
                block_hash: block_hash.clone(),
                uncles_hash: block.uncles_hash().raw_data().to_vec(),
            });

            for (idx, tx) in block.transactions().iter().enumerate() {
                let tx_hash = tx.hash().raw_data().to_vec();
                tx_table_batch.push(TransactionTable::from_view(
                    tx,
                    0,
                    idx as u16,
                    block_hash.clone(),
                    block_timestamp,
                    block_number,
                ));

                for (i, (cell, data)) in tx.outputs_with_data_iter().enumerate() {
                    let mut cell_table = CellTable::from_cell(
                        &cell,
                        0,
                        tx_hash.clone(),
                        i as u16,
                        idx as u16,
                        block_number,
                        block_hash.clone(),
                        block_epoch,
                    );

                    if data.len() < BIG_DATA_THRESHOLD {
                        cell_table.is_data_complete = true;
                        cell_table.data = data.to_vec();
                    } else {
                        big_data_table_batch.push(BigDataTable {
                            tx_hash: tx_hash.clone(),
                            output_index: i as u16,
                            data: data.to_vec(),
                        });
                    }

                    script_table_batch.insert(cell_table.to_lock_script_table(0));

                    if cell_table.has_type_script() {
                        script_table_batch.insert(cell_table.to_type_script_table(0));
                    }

                    cell_table_batch.push(cell_table);
                }

                tx.inputs()
                    .into_iter()
                    .for_each(|input| outpoint_tx.send(input.previous_output()).unwrap());
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
            big_data_table_batch
        );
    }

    number_tx.send(max_number).unwrap();

    Ok(())
}

pub async fn handle_out_point(
    mut conn: RBatisConnExecutor<'_>,
    rx: UnboundedReceiver<packed::OutPoint>,
) -> Result<()> {
    let mut queue = Vec::new();
    let mut stream = UnboundedReceiverStream::new(rx);

    while let Some(out_point) = stream.next().await {
        let tx_hash = out_point.tx_hash().raw_data().to_vec();
        let index: u32 = out_point.index().unpack();

        try_remove_live_cell(&mut conn, tx_hash, index as u16, &mut queue).await?;

        if queue.len() == 5000 {
            while let Some(item) = queue.pop() {
                try_remove_live_cell(&mut conn, item.tx_hash, item.index, &mut queue).await?;
            }
        }
    }

    Ok(())
}

async fn try_remove_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: Vec<u8>,
    index: u16,
    queue: &mut Vec<InnerOutPoint>,
) -> Result<()> {
    if sql::is_live_cell(conn, &tx_hash, index).await?.is_none() {
        queue.push(InnerOutPoint::new(tx_hash, index));
    } else {
        sql::remove_live_cell(conn, &tx_hash, index).await?;
    }

    Ok(())
}

struct InnerOutPoint {
    tx_hash: Vec<u8>,
    index: u16,
}

impl InnerOutPoint {
    fn new(tx_hash: Vec<u8>, index: u16) -> Self {
        InnerOutPoint { tx_hash, index }
    }
}
