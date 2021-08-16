use crate::table::{
    BigDataTable, BlockTable, BsonBytes, CanonicalChainTable, CellTable, LiveCellTable,
    ScriptTable, TransactionTable, UncleRelationshipTable,
};
use crate::{generate_id, insert::BIG_DATA_THRESHOLD, sql, to_bson_bytes, DBAdapter};

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
        let mut canonical_data_table_batch: Vec<CanonicalChainTable> = Vec::new();

        for block in blocks.iter() {
            let block_number = block.number();
            let block_hash = block.hash().raw_data().to_vec();
            let block_timestamp = block.timestamp();
            let block_epoch = block.epoch().full_value();
            max_number = max_number.max(block_number);

            block_table_batch.push(block.into());
            uncle_relationship_table_batch.push(UncleRelationshipTable {
                block_hash: to_bson_bytes(&block_hash),
                uncles_hash: to_bson_bytes(&block.uncles_hash().raw_data()),
            });
            canonical_data_table_batch.push(CanonicalChainTable {
                block_number,
                block_hash: to_bson_bytes(&block_hash),
            });

            for (idx, tx) in block.transactions().iter().enumerate() {
                let tx_hash = to_bson_bytes(&tx.hash().raw_data());
                tx_table_batch.push(TransactionTable::from_view(
                    tx,
                    generate_id(block_number),
                    idx as u16,
                    to_bson_bytes(&block_hash),
                    block_number,
                    block_timestamp,
                ));

                for (i, (cell, data)) in tx.outputs_with_data_iter().enumerate() {
                    let mut cell_table = CellTable::from_cell(
                        &cell,
                        generate_id(block_number),
                        tx_hash.clone(),
                        i as u16,
                        idx as u16,
                        block_number,
                        to_bson_bytes(&block_hash),
                        block_epoch,
                        true,
                        &[],
                    );

                    if data.len() < BIG_DATA_THRESHOLD {
                        cell_table.is_data_complete = true;
                        cell_table.data = to_bson_bytes(&data);
                    } else {
                        big_data_table_batch.push(BigDataTable {
                            tx_hash: tx_hash.clone(),
                            output_index: i as u16,
                            data: to_bson_bytes(&data),
                        });
                    }

                    script_table_batch
                        .insert(cell_table.to_lock_script_table(generate_id(block_number)));

                    if cell_table.has_type_script() {
                        script_table_batch
                            .insert(cell_table.to_type_script_table(generate_id(block_number)));
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
            big_data_table_batch,
            canonical_data_table_batch
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
    let mut threshold = MAX_OUT_POINT_QUEUE_SIZE;

    while let Some(out_point) = stream.next().await {
        let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
        let index: u32 = out_point.index().unpack();

        try_remove_live_cell(&mut conn, tx_hash, index as u16, &mut queue).await?;

        if queue.len() >= threshold {
            while let Some(item) = queue.pop() {
                try_remove_live_cell(
                    &mut conn,
                    to_bson_bytes(&item.tx_hash),
                    item.index,
                    &mut queue,
                )
                .await?;
            }

            threshold += 1000;
        }
    }

    Ok(())
}

async fn try_remove_live_cell(
    conn: &mut RBatisConnExecutor<'_>,
    tx_hash: BsonBytes,
    index: u16,
    queue: &mut Vec<InnerOutPoint>,
) -> Result<()> {
    if sql::is_live_cell(conn, tx_hash.clone(), index)
        .await?
        .is_none()
    {
        queue.push(InnerOutPoint::new(tx_hash.bytes.clone(), index));
    } else {
        sql::remove_live_cell(conn, tx_hash.clone(), index).await?;
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
