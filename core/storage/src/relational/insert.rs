use crate::relational::table::{
    CellTable, ConsumedInfo, IndexerCellTable, RegisteredAddressTable, BLAKE_160_HSAH_LEN,
    IO_TYPE_INPUT,
};
use crate::relational::{generate_id, to_rb_bytes, RelationalStorage};

use common::Result;
use common_logger::tracing_async;
use db_sqlx::SQLXPool;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor, Bytes as RbBytes};

use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::{prelude::*, H256};
use seq_macro::seq;
use sql_builder::SqlBuilder;
use sqlx::{Any, Row, Transaction};
use std::collections::HashSet;

pub const BATCH_SIZE_THRESHOLD: usize = 1_500;

#[macro_export]
macro_rules! save_batch_slice {
	($tx: expr$ (, $table: expr)*) => {{
		$(if $tx.save_batch_slice(&$table, BATCH_SIZE_THRESHOLD, &[]).await.is_err() {
            $tx.rollback().await?;
            return Ok(());
        })*
	}};
}

impl RelationalStorage {
    #[tracing_async]
    pub(crate) async fn insert_block_table(
        &self,
        block_view: &BlockView,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        // insert mercury_block
        insert_block(block_view, tx).await?;

        // insert mercury_sync_status
        SQLXPool::new_query(
            r#"
            INSERT INTO mercury_sync_status(block_number)
            VALUES ($1)
            "#,
        )
        .bind(i32::try_from(block_view.number())?)
        .execute(&mut *tx)
        .await?;

        // insert mercury_canonical_chain
        SQLXPool::new_query(
            r#"
            INSERT INTO mercury_canonical_chain(block_number, block_hash)
            VALUES ($1, $2)
            "#,
        )
        .bind(i32::try_from(block_view.number())?)
        .bind(block_view.hash().raw_data().to_vec())
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_transaction_table(
        &self,
        block_view: &BlockView,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        let block_number = block_view.number();
        let block_hash = block_view.hash().raw_data().to_vec();
        let block_timestamp = block_view.timestamp();
        let epoch = block_view.epoch();
        let tx_views = block_view.transactions();

        bulk_insert_transactions(block_number, &block_hash, block_timestamp, &tx_views, tx).await?;
        bulk_insert_output_cells(block_number, &block_hash, epoch, &tx_views, tx).await?;
        bulk_insert_scripts(&tx_views, tx).await?;
        update_consumed_cells(block_number, &block_hash, &tx_views, tx).await?;
        bulk_insert_indexer_cells(&tx_views, tx).await
    }

    async fn fill_and_save_indexer_cells(
        &self,
        block_number: u64,
        mut indexer_cells: Vec<IndexerCellTable>,
        infos: &[ConsumedInfo],
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        for info in infos.iter() {
            let tx_hash: H256 = info.out_point.tx_hash().unpack();
            let output_index: u32 = info.out_point.index().unpack();
            let w = self
                .pool
                .wrapper()
                .eq("tx_hash", to_rb_bytes(&tx_hash.0))
                .and()
                .eq("output_index", output_index);
            let cell_table = tx.fetch_by_wrapper::<CellTable>(w).await?;
            let indexer_cell = IndexerCellTable::new_with_empty_scripts(
                block_number,
                IO_TYPE_INPUT,
                info.input_index,
                info.consumed_tx_hash.clone(),
                info.consumed_tx_index,
            );
            indexer_cells.push(indexer_cell.update_by_cell_table(&cell_table));
        }

        indexer_cells.sort();
        indexer_cells
            .iter_mut()
            .for_each(|c| c.id = generate_id(block_number));

        save_batch_slice!(tx, indexer_cells);

        Ok(())
    }

    pub(crate) async fn insert_registered_address_table(
        &self,
        addresses: Vec<(RbBytes, String)>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<Vec<RbBytes>> {
        let mut res = vec![];
        for item in addresses {
            let (lock_hash, address) = item;
            if let Err(e) = tx
                .save(
                    &RegisteredAddressTable::new(lock_hash.clone(), address.clone()),
                    &[],
                )
                .await
            {
                if let Ok(Some(s)) = self.query_registered_address(&lock_hash).await {
                    if s != address {
                        return Err(e.into());
                    }
                } else {
                    return Err(e.into());
                }
            }
            res.push(lock_hash);
        }

        Ok(res)
    }
}

fn build_insert_sql(
    mut builder: SqlBuilder,
    column_number: usize,
    rows_number: usize,
) -> Result<String> {
    push_values_placeholders(&mut builder, column_number, rows_number);
    builder.sql().map(|s| s.trim_end_matches(';').to_string())
}

fn push_values_placeholders(builder: &mut SqlBuilder, column_number: usize, rows_number: usize) {
    let mut placeholder_idx = 1usize;
    for _ in 0..rows_number {
        let values = (placeholder_idx..placeholder_idx + column_number)
            .map(|i| format!("${}", i))
            .collect::<Vec<String>>();
        builder.values(&values);
        placeholder_idx += column_number;
    }
}

async fn insert_block(block_view: &BlockView, tx: &mut Transaction<'_, Any>) -> Result<()> {
    let block_row = (
        block_view.hash().raw_data().to_vec(),
        i32::try_from(block_view.number())?,
        i16::try_from(block_view.version())?,
        i32::try_from(block_view.compact_target())?,
        i64::try_from(block_view.timestamp())?,
        i32::try_from(block_view.epoch().number())?,
        i32::try_from(block_view.epoch().index())?,
        i32::try_from(block_view.epoch().length())?,
        block_view.parent_hash().raw_data().to_vec(),
        block_view.transactions_root().raw_data().to_vec(),
        block_view.proposals_hash().raw_data().to_vec(),
        block_view.extra_hash().raw_data().to_vec(),
        block_view.uncles().data().as_slice().to_vec(),
        i32::try_from(block_view.uncle_hashes().len())?,
        block_view.dao().raw_data().to_vec(),
        block_view.nonce().to_be_bytes().to_vec(),
        block_view.data().proposals().as_bytes().to_vec(),
    );

    // build query str
    let mut builder = SqlBuilder::insert_into("mercury_block");
    builder.field(
        "
        block_hash,
        block_number,
        version,
        compact_target,
        block_timestamp,
        epoch_number,
        epoch_index,
        epoch_length,
        parent_hash,
        transactions_root,
        proposals_hash,
        uncles_hash,
        uncles,
        uncles_count,
        dao,
        nonce,
        proposals",
    );
    let sql = build_insert_sql(builder, 17, 1)?;

    // bind
    let mut query = SQLXPool::new_query(&sql);
    seq!(i in 0..17 {
        query = query.bind(&block_row.i);
    });

    // execute
    query
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

async fn bulk_insert_transactions(
    block_number: u64,
    block_hash: &[u8],
    block_timestamp: u64,
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    let tx_rows: Vec<_> = tx_views
        .iter()
        .enumerate()
        .map(|(idx, transaction)| {
            (
                generate_id(block_number),
                transaction.hash().raw_data().to_vec(),
                idx as i32,
                transaction.inputs().len() as i32,
                transaction.outputs().len() as i32,
                block_number as i32,
                block_hash.to_vec(),
                block_timestamp as i64,
                transaction.version() as i16,
                transaction.cell_deps().as_bytes().to_vec(),
                transaction.header_deps().as_bytes().to_vec(),
                transaction.witnesses().as_bytes().to_vec(),
            )
        })
        .collect();

    for start in (0..tx_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(tx_rows.len());

        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_transaction");
        builder.field(
            "
                id, 
                tx_hash, 
                tx_index, 
                input_count, 
                output_count, 
                block_number, 
                block_hash, 
                tx_timestamp, 
                version, 
                cell_deps,         
                header_deps, 
                witnesses",
        );
        push_values_placeholders(&mut builder, 12, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in tx_rows.iter() {
            seq!(i in 0..12 {
                query = query.bind(&row.i);
            });
        }

        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}

async fn bulk_insert_output_cells(
    block_number: u64,
    block_hash: &[u8],
    epoch: EpochNumberWithFraction,
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    let mut output_cell_rows = Vec::new();

    for (tx_index, tx_view) in tx_views.iter().enumerate() {
        let tx_hash = tx_view.hash().raw_data();

        for (output_index, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let cell_capacity: u64 = cell.capacity().unpack();
            let cell_row = (
                generate_id(block_number),
                tx_hash.to_vec(),
                i32::try_from(output_index)?,
                i32::try_from(tx_index)?,
                block_hash.to_vec(),
                i32::try_from(block_number)?,
                i32::try_from(epoch.number())?,
                i32::try_from(epoch.index())?,
                i32::try_from(epoch.length())?,
                i64::try_from(cell_capacity)?,
                cell.lock().calc_script_hash().raw_data().to_vec(),
                cell.lock().code_hash().raw_data().to_vec(),
                cell.lock().args().raw_data().to_vec(),
                i16::try_from(u8::try_from(cell.lock().hash_type())?)?,
                if let Some(script) = cell.type_().to_opt() {
                    script.calc_script_hash().raw_data().to_vec()
                } else {
                    H256::default().0.to_vec()
                },
                if let Some(script) = cell.type_().to_opt() {
                    script.code_hash().raw_data().to_vec()
                } else {
                    Vec::<u8>::new()
                },
                if let Some(script) = cell.type_().to_opt() {
                    script.args().raw_data().to_vec()
                } else {
                    Vec::<u8>::new()
                },
                if let Some(script) = cell.type_().to_opt() {
                    i16::try_from(u8::try_from(script.hash_type())?)?
                } else {
                    0i16
                },
                data.to_vec(),
                Vec::<u8>::new(),
                Vec::<u8>::new(),
                Vec::<u8>::new(),
            );
            output_cell_rows.push(cell_row);
        }
    }

    // bulk insert
    for start in (0..output_cell_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(output_cell_rows.len());

        // mercury_cell
        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_cell");
        builder.field(
            r#"id,
            tx_hash,
            output_index,
            tx_index,
            block_hash,
            block_number,
            epoch_number,
            epoch_index,
            epoch_length,
            capacity,
            lock_hash,
            lock_code_hash,
            lock_args,
            lock_script_type,
            type_hash,
            type_code_hash,
            type_args,
            type_script_type,
            data,
            consumed_block_hash,
            consumed_tx_hash,
            since"#,
        );
        push_values_placeholders(&mut builder, 22, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in output_cell_rows.iter() {
            seq!(i in 0..22 {
                query = query.bind(&row.i);
            });
        }

        // execute
        query.execute(&mut *tx).await?;

        // mercury_live_cell
        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_live_cell");
        builder.field(
            r#"id,
            tx_hash,
            output_index,
            tx_index,
            block_hash,
            block_number,
            epoch_number,
            epoch_index,
            epoch_length,
            capacity,
            lock_hash,
            lock_code_hash,
            lock_args,
            lock_script_type,
            type_hash,
            type_code_hash,
            type_args,
            type_script_type,
            data
            "#,
        );
        push_values_placeholders(&mut builder, 19, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in output_cell_rows.iter() {
            seq!(i in 0..19 {
                query = query.bind(&row.i);
            });
        }

        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}

async fn bulk_insert_scripts(
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    let mut script_set = HashSet::new();
    let mut exist_script_cache = HashSet::new();

    for tx_view in tx_views.iter() {
        for (cell, _) in tx_view.outputs_with_data_iter() {
            if let Some(type_script) = cell.type_().to_opt() {
                let type_hash = type_script.calc_script_hash().raw_data();
                let type_script_args = type_script.args().raw_data();

                let type_script_row = (
                    type_hash.to_vec(),
                    type_hash.to_vec().split_at(BLAKE_160_HSAH_LEN).0.to_vec(),
                    type_script.code_hash().raw_data().to_vec(),
                    type_script_args.to_vec(),
                    i16::try_from(u8::try_from(type_script.hash_type())?)?,
                    i32::try_from(type_script_args.to_vec().len())?,
                );
                if !script_set.contains(&type_script_row)
                    && !script_exists(&type_script_row.0, &mut exist_script_cache, tx).await?
                {
                    exist_script_cache.insert(type_script_row.0.clone());
                    script_set.insert(type_script_row);
                }
            }

            let lock_script = cell.lock();
            let lock_hash = lock_script.calc_script_hash().raw_data();
            let lock_script_args = lock_script.args().raw_data();
            let lock_script_row = (
                lock_hash.to_vec(),
                lock_hash.to_vec().split_at(BLAKE_160_HSAH_LEN).0.to_vec(),
                lock_script.code_hash().raw_data().to_vec(),
                lock_script_args.to_vec(),
                i16::try_from(u8::try_from(lock_script.hash_type())?)?,
                i32::try_from(lock_script_args.to_vec().len())?,
            );
            if !script_set.contains(&lock_script_row)
                && !script_exists(&lock_script_row.0, &mut exist_script_cache, tx).await?
            {
                exist_script_cache.insert(lock_script_row.0.clone());
                script_set.insert(lock_script_row);
            }
        }
    }

    let script_rows = script_set.iter().cloned().collect::<Vec<_>>();

    // bulk insert
    for start in (0..script_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(script_rows.len());

        // build query str
        let mut builder = SqlBuilder::insert_into("mercury_script");
        builder.field(
            r#"script_hash,
            script_hash_160,
            script_code_hash,
            script_args,
            script_type,
            script_args_len"#,
        );
        push_values_placeholders(&mut builder, 6, end - start);
        let sql = builder.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&sql);
        for row in script_rows.iter() {
            seq!(i in 0..6 {
                query = query.bind(&row.i);
            });
        }
        // execute
        query.execute(&mut *tx).await?;
    }

    Ok(())
}

async fn update_consumed_cells(
    consumed_block_number: u64,
    consumed_block_hash: &[u8],
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    for (tx_index, transaction) in tx_views.iter().enumerate() {
        if tx_index == 0 {
            continue;
        }

        let consumed_tx_hash = transaction.hash().raw_data().to_vec();

        for (input_index, input) in transaction.inputs().into_iter().enumerate() {
            let since: u64 = input.since().unpack();
            let previous_output_tx_hash = input.previous_output().tx_hash().raw_data().to_vec();
            let previous_output_index: u32 = input.previous_output().index().unpack();

            sqlx::query(
                r#"DELETE FROM mercury_live_cell 
                WHERE tx_hash = $1 AND output_index = $2
                "#,
            )
            .bind(&previous_output_tx_hash)
            .bind(previous_output_index as i32)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"UPDATE mercury_cell SET
                consumed_block_number = $1, 
                consumed_block_hash = $2,
                consumed_tx_hash = $3, 
                consumed_tx_index = $4, 
                input_index = $5, 
                since = $6
                WHERE tx_hash = $7 AND output_index = $8
                "#,
            )
            .bind(consumed_block_number as i64)
            .bind(consumed_block_hash)
            .bind(&consumed_tx_hash)
            .bind(tx_index as i32)
            .bind(input_index as i32)
            .bind(since.to_be_bytes().as_slice())
            .bind(&previous_output_tx_hash)
            .bind(previous_output_index as i32)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn bulk_insert_indexer_cells(
    _tx_views: &[TransactionView],
    _tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    Ok(())
}

async fn script_exists(
    script_hash: &[u8],
    exist_script_cache: &mut HashSet<Vec<u8>>,
    tx: &mut Transaction<'_, Any>,
) -> Result<bool> {
    if exist_script_cache.contains(script_hash) {
        return Ok(true);
    }

    let row = sqlx::query(
        "SELECT COUNT(*) as count 
        FROM mercury_script WHERE
        script_hash = $1",
    )
    .bind(script_hash)
    .fetch_one(tx)
    .await?;

    Ok(row.get::<i64, _>("count") != 0)
}
