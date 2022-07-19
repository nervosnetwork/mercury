use crate::relational::table::{
    BlockTable, CanonicalChainTable, CellTable, ConsumedInfo, ConsumedInfo_, IndexerCellTable,
    LiveCellTable, RegisteredAddressTable, ScriptTable, SyncStatus, TransactionTable,
    IO_TYPE_INPUT, IO_TYPE_OUTPUT,
};
use crate::relational::{generate_id, sql, to_rb_bytes, RelationalStorage};

use common::{Context, Result};
use common_logger::tracing_async;
use db_sqlx::SQLXPool;
use db_xsql::rbatis::{crud::CRUDMut, executor::RBatisTxExecutor, Bytes as RbBytes};

use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::{prelude::*, H256};
use seq_macro::seq;
use sql_builder::SqlBuilder;
use sqlx::{Any, Transaction};

use std::collections::{HashMap, HashSet};

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
        _ctx: Context,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());

        tx.save(&BlockTable::from(block_view), &[]).await?;
        tx.save(&SyncStatus::new(block_view.number()), &[]).await?;
        tx.save(
            &CanonicalChainTable::new(block_view.number(), block_hash),
            &[],
        )
        .await?;

        Ok(())
    }

    #[tracing_async]
    pub(crate) async fn insert_block_table_(
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

    #[tracing_async]
    pub(crate) async fn insert_transaction_table(
        &self,
        _ctx: Context,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        let txs = block_view.transactions();
        let block_number = block_view.number();
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());
        let block_timestamp = block_view.timestamp();
        let mut output_cell_set = Vec::new();
        let mut live_cell_set = Vec::new();
        let mut tx_set = Vec::new();
        let mut script_set = HashSet::new();
        let mut consumed_infos = Vec::new();
        let mut indexer_cells = Vec::new();

        for (idx, transaction) in txs.iter().enumerate() {
            let index = idx as u32;

            tx_set.push(TransactionTable::from_view(
                transaction,
                generate_id(block_number),
                index,
                block_hash.clone(),
                block_number,
                block_timestamp,
            ));

            self.insert_cell_table(
                transaction,
                index,
                block_view,
                tx,
                &mut output_cell_set,
                &mut live_cell_set,
                &mut script_set,
                &mut consumed_infos,
                &mut indexer_cells,
            )
            .await?;
        }

        let script_batch = script_set.iter().cloned().collect::<Vec<_>>();
        save_batch_slice!(tx, tx_set, output_cell_set, live_cell_set, script_batch);

        self.update_consumed_cells(&consumed_infos, tx).await?;
        self.fill_and_save_indexer_cells(block_number, indexer_cells, &consumed_infos, tx)
            .await?;

        Ok(())
    }

    pub(crate) async fn insert_transaction_table_(
        &self,
        block_view: &BlockView,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<()> {
        let tx_views = block_view.transactions();
        let block_number = block_view.number();
        let block_hash = block_view.hash().raw_data().to_vec();
        let block_timestamp = block_view.timestamp();

        bulk_insert_transactions(block_number, block_hash, block_timestamp, &tx_views, tx).await?;
        bulk_insert_output_cells(block_view, &tx_views, tx).await?;
        bulk_update_consumed_cells(&tx_views, tx).await?;
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

    async fn update_consumed_cells(
        &self,
        infos: &[ConsumedInfo],
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<()> {
        for info in infos.iter() {
            let tx_hash = to_rb_bytes(&info.out_point.tx_hash().raw_data());
            let output_index: u32 = info.out_point.index().unpack();

            self.remove_live_cell_by_out_point(&info.out_point, tx)
                .await?;
            sql::update_consume_cell(
                tx,
                &info.consumed_block_number,
                &info.consumed_block_hash.clone(),
                &info.consumed_tx_hash.clone(),
                &info.consumed_tx_index,
                &info.input_index,
                &info.since.clone(),
                &tx_hash,
                &output_index,
            )
            .await?;
        }

        Ok(())
    }

    async fn insert_cell_table(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_view: &BlockView,
        tx: &mut RBatisTxExecutor<'_>,
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
        consumed_infos: &mut Vec<ConsumedInfo>,
        indexer_cells: &mut Vec<IndexerCellTable>,
    ) -> Result<()> {
        let block_hash = to_rb_bytes(&block_view.hash().raw_data());
        let block_number = block_view.number();
        let epoch = block_view.epoch();

        if tx_index > 0 {
            self.collect_consume_input_cells(
                tx_view,
                block_number,
                block_hash.clone(),
                tx_index,
                consumed_infos,
            )
            .await?;
        }

        self.insert_output_cells(
            tx_view,
            tx_index,
            block_number,
            block_hash.clone(),
            epoch,
            tx,
            output_cell_set,
            live_cell_set,
            script_set,
            indexer_cells,
        )
        .await?;

        Ok(())
    }

    async fn insert_output_cells(
        &self,
        tx_view: &TransactionView,
        tx_index: u32,
        block_number: u64,
        block_hash: RbBytes,
        epoch: EpochNumberWithFraction,
        tx: &mut RBatisTxExecutor<'_>,
        output_cell_set: &mut Vec<CellTable>,
        live_cell_set: &mut Vec<LiveCellTable>,
        script_set: &mut HashSet<ScriptTable>,
        indexer_cell_set: &mut Vec<IndexerCellTable>,
    ) -> Result<()> {
        let tx_hash = to_rb_bytes(&tx_view.hash().raw_data());
        let mut has_script_cache = HashMap::new();

        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let mut table = CellTable::from_cell(
                &cell,
                generate_id(block_number),
                tx_hash.clone(),
                idx as u32,
                tx_index,
                block_number,
                block_hash.clone(),
                epoch,
                &data,
            );

            if let Some(type_script) = cell.type_().to_opt() {
                table.set_type_script_info(&type_script);
                let type_script_table = table.to_type_script_table();

                if !script_set.contains(&type_script_table)
                    && !self
                        .has_script(&type_script_table, &mut has_script_cache, tx)
                        .await?
                {
                    script_set.insert(type_script_table);
                }
            }

            let lock_table = table.to_lock_script_table();
            if !script_set.contains(&lock_table)
                && !self
                    .has_script(&lock_table, &mut has_script_cache, tx)
                    .await?
            {
                script_set.insert(lock_table);
            }

            let indexer_cell = IndexerCellTable::new_with_empty_scripts(
                block_number,
                IO_TYPE_OUTPUT,
                idx as u32,
                tx_hash.clone(),
                tx_index,
            );

            indexer_cell_set.push(indexer_cell.update_by_cell_table(&table));
            output_cell_set.push(table.clone());
            live_cell_set.push(table.into());
        }

        Ok(())
    }

    async fn collect_consume_input_cells(
        &self,
        tx_view: &TransactionView,
        consumed_block_number: u64,
        consumed_block_hash: RbBytes,
        tx_index: u32,
        consumed_infos: &mut Vec<ConsumedInfo>,
    ) -> Result<()> {
        let consumed_tx_hash = to_rb_bytes(&tx_view.hash().raw_data());

        for (idx, input) in tx_view.inputs().into_iter().enumerate() {
            let since: u64 = input.since().unpack();

            consumed_infos.push(ConsumedInfo {
                out_point: input.previous_output(),
                input_index: idx as u32,
                consumed_block_hash: consumed_block_hash.clone(),
                consumed_block_number,
                consumed_tx_hash: consumed_tx_hash.clone(),
                consumed_tx_index: tx_index,
                since: to_rb_bytes(&since.to_be_bytes()),
            });
        }

        Ok(())
    }

    async fn collect_consume_input_cells_(
        &self,
        tx_view: &TransactionView,
        consumed_block_number: u64,
        consumed_block_hash: &[u8],
        tx_index: u32,
    ) -> Option<Vec<ConsumedInfo_>> {
        if tx_index == 0 {
            return None;
        }

        let consumed_tx_hash = tx_view.hash().raw_data().to_vec();
        Some(
            tx_view
                .inputs()
                .into_iter()
                .enumerate()
                .map(|(idx, input)| ConsumedInfo_ {
                    out_point: input.previous_output(),
                    input_index: idx as u32,
                    consumed_block_hash: consumed_block_hash.to_vec(),
                    consumed_block_number,
                    consumed_tx_hash: consumed_tx_hash.clone(),
                    consumed_tx_index: tx_index,
                    since: input.since().unpack(),
                })
                .collect(),
        )
    }

    async fn has_script(
        &self,
        table: &ScriptTable,
        has_script_cache: &mut HashMap<Vec<u8>, bool>,
        tx: &mut RBatisTxExecutor<'_>,
    ) -> Result<bool> {
        if let Some(res) = has_script_cache.get(&table.script_hash.inner) {
            if *res {
                return Ok(true);
            }
        }

        let w = self
            .pool
            .wrapper()
            .eq("script_hash", table.script_hash.clone());
        let res = tx.fetch_count_by_wrapper::<ScriptTable>(w).await?;
        let ret = res != 0;

        has_script_cache
            .entry(table.script_hash.inner.clone())
            .or_insert(ret);

        Ok(ret)
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
    block_hash: Vec<u8>,
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
    block_view: &BlockView,
    tx_views: &[TransactionView],
    tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    let block_number = block_view.number();
    let block_hash = block_view.hash().raw_data();
    let epoch = block_view.epoch();

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

    // bulk_insert_cells
    for start in (0..output_cell_rows.len()).step_by(BATCH_SIZE_THRESHOLD) {
        let end = (start + BATCH_SIZE_THRESHOLD).min(output_cell_rows.len());

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
    }

    Ok(())
}

async fn bulk_update_consumed_cells(
    _tx_views: &[TransactionView],
    _tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    Ok(())
}

async fn bulk_insert_indexer_cells(
    _tx_views: &[TransactionView],
    _tx: &mut Transaction<'_, Any>,
) -> Result<()> {
    Ok(())
}
