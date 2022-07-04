use crate::error::DBError;
use crate::relational::table::{
    decode_since, CanonicalChainTable, CellTable, IndexerCellTable, LiveCellTable,
    RegisteredAddressTable, ScriptTable,
};
use crate::relational::{to_rb_bytes, RelationalStorage};

use common::Order;
use common::{
    utils, utils::to_fixed_array, Context, DetailedCell, PaginationRequest, PaginationResponse,
    Range, Result,
};
use common_logger::tracing_async;
use db_sqlx::SQLXPool;
use db_xsql::page::PageRequest;
use db_xsql::rbatis::{plugin::page::Page, Bytes as RbBytes};
use protocol::db::{SimpleBlock, SimpleTransaction, TransactionWrapper};
use sqlx::{any::AnyRow, Row};

use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::bytes::Bytes;
use ckb_types::core::{
    BlockBuilder, BlockNumber, BlockView, EpochNumberWithFraction, HeaderBuilder, HeaderView,
    TransactionBuilder, TransactionView, UncleBlockView,
};
use ckb_types::{packed, prelude::*, H256};
use sql_builder::SqlBuilder;
use sqlx::{query::Query, Any, IntoArguments};

use std::collections::HashMap;
use std::convert::From;
use std::slice::Iter;

macro_rules! build_next_cursor {
    ($page: expr, $pagination: expr) => {{
        if $page.records.is_empty()
            || $page.search_count
                && $page.total <= $pagination.limit.map(Into::into).unwrap_or(u64::MAX)
            || ($page.records.len() as u64) < $pagination.limit.map(Into::into).unwrap_or(u64::MAX)
        {
            None
        } else {
            Some($page.records.last().cloned().unwrap().id as u64)
        }
    }};
}

impl RelationalStorage {
    pub(crate) async fn query_tip(&self) -> Result<Option<(BlockNumber, H256)>> {
        let query = SQLXPool::new_query(
            r#"
            SELECT * FROM mercury_canonical_chain
            ORDER BY block_number
            DESC
            "#,
        );
        let res = self.sqlx_pool.fetch_optional(query).await?;
        if let Some(row) = res {
            Ok(Some((
                row.get::<i32, _>("block_number") as u64,
                bytes_to_h256(&row.get::<Vec<u8>, _>("block_hash")),
            )))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_block_by_number(
        &self,
        ctx: Context,
        block_number: BlockNumber,
    ) -> Result<BlockView> {
        let block = self
            .query_block_by_number(block_number.try_into().map_err(|_| DBError::WrongHeight)?)
            .await?;
        self.get_block_view(ctx, &block).await
    }

    pub(crate) async fn get_block_by_hash(
        &self,
        ctx: Context,
        block_hash: H256,
    ) -> Result<BlockView> {
        let block = self.query_block_by_hash(block_hash.as_bytes()).await?;
        self.get_block_view(ctx, &block).await
    }

    pub(crate) async fn get_tip_block(&self, ctx: Context) -> Result<BlockView> {
        let block = self.query_tip_block().await?;
        self.get_block_view(ctx, &block).await
    }

    pub(crate) async fn get_tip_block_header(&self) -> Result<HeaderView> {
        let block = self.query_tip_block().await?;
        Ok(build_header_view(&block))
    }

    pub(crate) async fn get_block_header_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<HeaderView> {
        let block = self.query_block_by_hash(block_hash.as_bytes()).await?;
        Ok(build_header_view(&block))
    }

    pub(crate) async fn get_block_header_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<HeaderView> {
        let block = self
            .query_block_by_number(block_number.try_into().map_err(|_| DBError::WrongHeight)?)
            .await?;
        Ok(build_header_view(&block))
    }

    async fn get_block_view(&self, ctx: Context, block: &AnyRow) -> Result<BlockView> {
        let header = build_header_view(block);
        let uncles = packed::UncleBlockVec::from_slice(&block.get::<Vec<u8>, _>("uncles"))?
            .into_iter()
            .map(|uncle| uncle.into_view())
            .collect::<Vec<_>>();
        let txs = self
            .get_transactions_by_block_hash(ctx, &block.get::<Vec<u8>, _>("block_hash"))
            .await?;
        let proposals = build_proposals(block.get::<Vec<u8>, _>("proposals"));
        Ok(build_block_view(header, uncles, txs, proposals))
    }

    async fn get_transactions_by_block_hash(
        &self,
        ctx: Context,
        block_hash: &[u8],
    ) -> Result<Vec<TransactionView>> {
        let txs = self.query_transactions_by_block_hash(block_hash).await?;
        self.get_transaction_views(ctx, txs).await
    }

    pub(crate) async fn query_simple_transaction(
        &self,
        tx_hash: H256,
    ) -> Result<SimpleTransaction> {
        let query = SQLXPool::new_query(
            r#"
            SELECT tx_index, block_number, block_hash, epoch_number, epoch_index, epoch_length 
            FROM mercury_cell
            WHERE tx_hash = $1
            "#,
        )
        .bind(tx_hash.as_bytes());
        let cell = self.sqlx_pool.fetch_one(query).await?;
        let epoch_number = EpochNumberWithFraction::new(
            cell.get::<i32, _>("epoch_number").try_into()?,
            cell.get::<i32, _>("epoch_index").try_into()?,
            cell.get::<i32, _>("epoch_length").try_into()?,
        )
        .to_rational();
        let block_hash = bytes_to_h256(&cell.get::<Vec<u8>, _>("block_hash"));
        let block_number = cell.get::<i32, _>("block_number").try_into()?;
        let tx_index = cell.get::<i32, _>("tx_index").try_into()?;
        Ok(SimpleTransaction {
            epoch_number,
            block_number,
            block_hash,
            tx_index,
        })
    }

    pub(crate) async fn query_spent_tx_hash(
        &self,
        out_point: packed::OutPoint,
    ) -> Result<Option<H256>> {
        let tx_hash: H256 = out_point.tx_hash().unpack();
        let output_index: u32 = out_point.index().unpack();
        let query = SQLXPool::new_query(
            r#"
            SELECT tx_hash, output_index, consumed_tx_hash
            FROM mercury_cell
            WHERE tx_hash = $1 AND output_index = $2
            "#,
        )
        .bind(tx_hash.as_bytes())
        .bind(i32::try_from(output_index)?);
        let cell = self.sqlx_pool.fetch_optional(query).await?;
        if let Some(cell) = cell {
            let consumed_tx_hash = cell.get::<Vec<u8>, _>("consumed_tx_hash");
            if consumed_tx_hash.is_empty() {
                Ok(None)
            } else {
                Ok(Some(bytes_to_h256(&consumed_tx_hash)))
            }
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_transaction_views(
        &self,
        ctx: Context,
        txs: Vec<AnyRow>,
    ) -> Result<Vec<TransactionView>> {
        let txs_wrapper = self.get_transactions_with_status(ctx, txs).await?;
        let tx_views = txs_wrapper
            .into_iter()
            .map(|tx_wrapper| tx_wrapper.transaction_view)
            .collect();
        Ok(tx_views)
    }

    #[tracing_async]
    pub(crate) async fn get_transactions_with_status(
        &self,
        _ctx: Context,
        txs: Vec<AnyRow>,
    ) -> Result<Vec<TransactionWrapper>> {
        if txs.is_empty() {
            return Ok(Vec::new());
        }

        let tx_hashes: Vec<RbBytes> = txs
            .iter()
            .map(|tx| tx.get::<Vec<u8>, _>("tx_hash").into())
            .collect();
        let output_cells = self.query_txs_output_cells(&tx_hashes).await?;
        let input_cells = self.query_txs_input_cells(&tx_hashes).await?;

        let mut output_cells_group_by_tx_hash = HashMap::new();
        for cell in output_cells {
            output_cells_group_by_tx_hash
                .entry(cell.tx_hash.inner.clone())
                .or_insert_with(Vec::new)
                .push(cell);
        }

        let mut input_cells_group_by_tx_hash = HashMap::new();
        for cell in input_cells {
            input_cells_group_by_tx_hash
                .entry(cell.consumed_tx_hash.inner.clone())
                .or_insert_with(Vec::new)
                .push(cell);
        }

        let txs_with_status = txs
            .into_iter()
            .map(|tx| {
                let witnesses = build_witnesses(tx.get::<Vec<u8>, _>("witnesses"));
                let header_deps = build_header_deps(tx.get::<Vec<u8>, _>("header_deps"));
                let cell_deps = build_cell_deps(tx.get::<Vec<u8>, _>("cell_deps"));

                let input_tables = input_cells_group_by_tx_hash
                    .get(&tx.get::<Vec<u8>, _>("tx_hash"))
                    .cloned()
                    .unwrap_or_default();
                let mut inputs = build_cell_inputs(input_tables.iter());
                if inputs.is_empty() && tx.get::<i32, _>("tx_index") == 0 {
                    inputs = vec![build_cell_base_input(
                        tx.get::<i32, _>("block_number")
                            .try_into()
                            .expect("i32 to u64"),
                    )]
                };

                let output_tables = output_cells_group_by_tx_hash
                    .get(&tx.get::<Vec<u8>, _>("tx_hash"))
                    .cloned()
                    .unwrap_or_default();
                let (outputs, outputs_data) = build_cell_outputs(&output_tables);

                let transaction_view = build_transaction_view(
                    tx.get::<i16, _>("version").try_into().expect("i16 to u32"),
                    witnesses,
                    inputs,
                    outputs,
                    outputs_data,
                    cell_deps,
                    header_deps,
                );
                let transaction_with_status = TransactionWithStatus::with_committed(
                    Some(transaction_view.clone()),
                    bytes_to_h256(&tx.get::<Vec<u8>, _>("block_hash")),
                );

                let is_cellbase = tx.get::<i32, _>("tx_index") == 0;
                let input_cells = input_tables.into_iter().map(Into::into).collect();
                let output_cells = output_tables.into_iter().map(Into::into).collect();
                let timestamp = tx
                    .get::<i64, _>("tx_timestamp")
                    .try_into()
                    .expect("i64 to u64");

                TransactionWrapper {
                    transaction_with_status,
                    transaction_view,
                    input_cells,
                    output_cells,
                    is_cellbase,
                    timestamp,
                }
            })
            .collect();
        Ok(txs_with_status)
    }

    pub(crate) async fn get_tip_simple_block(&self) -> Result<SimpleBlock> {
        let (block_hash, block_number, parent_hash, block_timestamp) =
            self.query_tip_simple_block().await?;
        self.get_simple_block(block_hash, block_number, parent_hash, block_timestamp)
            .await
    }

    pub(crate) async fn get_simple_block_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<SimpleBlock> {
        let (block_hash, block_number, parent_hash, block_timestamp) = self
            .query_simple_block_by_number(
                block_number.try_into().map_err(|_| DBError::WrongHeight)?,
            )
            .await?;
        self.get_simple_block(block_hash, block_number, parent_hash, block_timestamp)
            .await
    }

    pub(crate) async fn get_simple_block_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<SimpleBlock> {
        let (block_hash, block_number, parent_hash, block_timestamp) = self
            .query_simple_block_by_hash(block_hash.as_bytes())
            .await?;
        self.get_simple_block(block_hash, block_number, parent_hash, block_timestamp)
            .await
    }

    async fn get_simple_block(
        &self,
        block_hash: H256,
        block_number: BlockNumber,
        parent_hash: H256,
        timestamp: u64,
    ) -> Result<SimpleBlock> {
        let transactions = self
            .query_transaction_hashes_by_block_hash(block_hash.as_bytes())
            .await?;
        Ok(SimpleBlock {
            block_number,
            block_hash,
            parent_hash,
            timestamp,
            transactions,
        })
    }

    pub(crate) async fn query_scripts(
        &self,
        script_hashes: Vec<Vec<u8>>,
        code_hashes: Vec<Vec<u8>>,
        args_len: Option<usize>,
        args: Vec<Vec<u8>>,
    ) -> Result<Vec<packed::Script>> {
        if script_hashes.is_empty() && args.is_empty() {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query scripts".to_owned(),
            )
            .into());
        }

        // build query str
        let mut query_builder = SqlBuilder::select_from("mercury_script");
        let mut query = query_builder.field("script_code_hash, script_args, script_type");
        if !script_hashes.is_empty() {
            query = query.and_where_in(
                "script_hash_160",
                &sqlx_param_placeholders(1..script_hashes.len())?,
            );
        }
        if !code_hashes.is_empty() {
            query = query.and_where_in(
                "script_code_hash",
                &sqlx_param_placeholders(
                    script_hashes.len() + 1..script_hashes.len() + code_hashes.len(),
                )?,
            );
        }
        if !args.is_empty() {
            query = query.and_where_in(
                "script_args",
                &sqlx_param_placeholders(
                    script_hashes.len() + code_hashes.len() + 1
                        ..script_hashes.len() + code_hashes.len() + args.len(),
                )?,
            );
        }
        if let Some(len) = args_len {
            query = query.and_where_eq("script_args_len", len);
        }
        let query = query.sql()?.trim_end_matches(';').to_string();

        // bind
        let mut query = SQLXPool::new_query(&query);
        for script_hash in &script_hashes {
            query = query.bind(script_hash);
        }
        for code_hash in &code_hashes {
            query = query.bind(code_hash);
        }
        for arg in &args {
            query = query.bind(arg);
        }

        // fetch
        let res = self.sqlx_pool.fetch(query).await?;
        Ok(res
            .into_iter()
            .map(|row| {
                packed::ScriptBuilder::default()
                    .code_hash(bytes_to_h256(&row.get::<Vec<u8>, _>("script_code_hash")).pack())
                    .args(row.get::<Vec<u8>, _>("script_args").pack())
                    .hash_type(packed::Byte::new(row.get::<i16, _>("script_type") as u8))
                    .build()
            })
            .collect())
    }

    pub(crate) async fn query_canonical_block_hash(
        &self,
        block_number: BlockNumber,
    ) -> Result<H256> {
        let ret = self
            .pool
            .fetch_by_column::<CanonicalChainTable, u64>("block_number", &block_number)
            .await?;
        Ok(rb_bytes_to_h256(&ret.block_hash))
    }

    async fn query_live_cell_by_out_point(
        &self,
        out_point: packed::OutPoint,
    ) -> Result<DetailedCell> {
        let tx_hash: H256 = out_point.tx_hash().unpack();
        let output_index: u32 = out_point.index().unpack();
        let query = SQLXPool::new_query(
            r#"
            SELECT *
            FROM mercury_live_cell
            WHERE tx_hash = $1 AND output_index = $2
            "#,
        )
        .bind(tx_hash.as_bytes())
        .bind(i32::try_from(output_index)?);
        let row = self.sqlx_pool.fetch_one(query).await?;
        build_detailed_cell(row)
    }

    async fn query_cell_by_out_point(&self, out_point: packed::OutPoint) -> Result<DetailedCell> {
        let tx_hash: H256 = out_point.tx_hash().unpack();
        let output_index: u32 = out_point.index().unpack();
        let query = SQLXPool::new_query(
            r#"
            SELECT *
            FROM mercury_cell
            WHERE tx_hash = $1 AND output_index = $2
            "#,
        )
        .bind(tx_hash.as_bytes())
        .bind(i32::try_from(output_index)?);
        let row = self.sqlx_pool.fetch_one(query).await?;
        build_detailed_cell(row)
    }

    #[tracing_async]
    pub(crate) async fn query_live_cells(
        &self,
        _ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<RbBytes>,
        type_hashes: Vec<RbBytes>,
        block_range: Option<Range>,
        capacity_range: Option<Range>,
        data_len_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        if lock_hashes.is_empty()
            && type_hashes.is_empty()
            && block_range.is_none()
            && out_point.is_none()
        {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query live cells".to_owned(),
            )
            .into());
        }

        if let Some(op) = out_point {
            let cell = self.query_live_cell_by_out_point(op).await?;

            let mut is_ok = true;
            let lock_hash: H256 = cell.cell_output.lock().calc_script_hash().unpack();
            let lock_hash = to_rb_bytes(&lock_hash.0);
            if !lock_hashes.is_empty() {
                is_ok &= lock_hashes.contains(&lock_hash)
            };

            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                let type_hash: H256 = type_script.calc_script_hash().unpack();
                let type_hash = to_rb_bytes(&type_hash.0);
                if !type_hashes.is_empty() {
                    is_ok &= type_hashes.contains(&type_hash)
                };
            } else if !type_hashes.is_empty() {
                let default_hashes = vec![H256::default()]
                    .into_iter()
                    .map(|hash| to_rb_bytes(&hash.0))
                    .collect::<Vec<_>>();
                is_ok &= type_hashes == default_hashes
            }

            if let Some(range) = block_range {
                is_ok &= range.is_in(cell.block_number);
            }

            if let Some(range) = capacity_range {
                is_ok &= range.is_in(cell.cell_output.capacity().unpack())
            }

            if let Some(range) = data_len_range {
                is_ok &= range.is_in(cell.cell_data.len() as u64)
            }

            let mut response: Vec<DetailedCell> = vec![];
            if is_ok {
                response.push(cell);
            }
            let count = response.len() as u64;
            return Ok(PaginationResponse {
                response,
                next_cursor: None,
                count: if pagination.return_count {
                    Some(count)
                } else {
                    None
                },
            });
        }

        let mut wrapper = self.pool.wrapper();

        if !lock_hashes.is_empty() {
            wrapper = wrapper.in_array("lock_hash", &lock_hashes);
        }

        if !type_hashes.is_empty() {
            wrapper = wrapper.and().in_array("type_hash", &type_hashes);
        }

        if let Some(range) = block_range {
            wrapper = wrapper
                .and()
                .between("block_number", range.min(), range.max())
        }

        if let Some(range) = capacity_range {
            wrapper = wrapper.and().between("capacity", range.min(), range.max())
        }

        if let Some(range) = data_len_range {
            wrapper = wrapper
                .and()
                .between("LENGTH(data)", range.min(), range.max())
        }

        let cells: Page<LiveCellTable> = self
            .pool
            .fetch_page_by_wrapper(wrapper, &PageRequest::from(pagination.clone()))
            .await?;
        let next_cursor = build_next_cursor!(cells, pagination);
        let res = cells
            .records
            .into_iter()
            .map(|r| {
                let cell: CellTable = r.into();
                cell.into()
            })
            .collect();
        Ok(to_pagination_response(
            res,
            next_cursor,
            if pagination.return_count {
                Some(cells.total)
            } else {
                None
            },
        ))
    }

    #[tracing_async]
    pub(crate) async fn query_cells(
        &self,
        _ctx: Context,
        out_point: Option<packed::OutPoint>,
        lock_hashes: Vec<RbBytes>,
        type_hashes: Vec<RbBytes>,
        block_range: Option<Range>,
        limit_cellbase: bool,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        if lock_hashes.is_empty()
            && type_hashes.is_empty()
            && block_range.is_none()
            && out_point.is_none()
        {
            return Err(
                DBError::InvalidParameter("no valid parameter to query cells".to_owned()).into(),
            );
        }

        if let Some(op) = out_point {
            let cell = self.query_cell_by_out_point(op).await?;

            let mut is_ok = true;
            let lock_hash: H256 = cell.cell_output.lock().calc_script_hash().unpack();
            let lock_hash = to_rb_bytes(&lock_hash.0);
            if !lock_hashes.is_empty() {
                is_ok &= lock_hashes.contains(&lock_hash)
            };

            if let Some(type_script) = cell.cell_output.type_().to_opt() {
                let type_hash: H256 = type_script.calc_script_hash().unpack();
                let type_hash = to_rb_bytes(&type_hash.0);
                if !type_hashes.is_empty() {
                    is_ok &= type_hashes.contains(&type_hash)
                };
            } else if !type_hashes.is_empty() {
                let default_hashes = vec![H256::default()]
                    .into_iter()
                    .map(|hash| to_rb_bytes(&hash.0))
                    .collect::<Vec<_>>();
                is_ok &= type_hashes == default_hashes
            }

            if let Some(range) = block_range {
                is_ok &= range.is_in(cell.block_number);
            }

            if limit_cellbase {
                is_ok &= cell.tx_index == 0;
            }

            let mut response: Vec<DetailedCell> = vec![];
            if is_ok {
                response.push(cell);
            }
            let count = response.len() as u64;
            return Ok(PaginationResponse {
                response,
                next_cursor: None,
                count: if pagination.return_count {
                    Some(count)
                } else {
                    None
                },
            });
        }

        let mut wrapper = self.pool.wrapper();

        if !lock_hashes.is_empty() {
            wrapper = wrapper.in_array("lock_hash", &lock_hashes);
        }

        if !type_hashes.is_empty() {
            wrapper = wrapper.and().in_array("type_hash", &type_hashes);
        }

        if let Some(range) = block_range {
            wrapper = wrapper
                .and()
                .push_sql("(")
                .between("block_number", range.min(), range.max())
                .or()
                .between("consumed_block_number", range.min(), range.max())
                .push_sql(")");
        }

        if limit_cellbase {
            wrapper = wrapper.and().eq("tx_index", 0)
        }

        let cells: Page<CellTable> = self
            .pool
            .fetch_page_by_wrapper(wrapper, &PageRequest::from(pagination.clone()))
            .await?;
        let next_cursor = build_next_cursor!(cells, pagination);
        let res = cells.records.into_iter().map(Into::into).collect();
        Ok(to_pagination_response(
            res,
            next_cursor,
            if pagination.return_count {
                Some(cells.total)
            } else {
                None
            },
        ))
    }

    #[tracing_async]
    pub(crate) async fn query_historical_live_cells(
        &self,
        _ctx: Context,
        lock_hashes: Vec<RbBytes>,
        type_hashes: Vec<RbBytes>,
        tip_block_number: u64,
        out_point: Option<packed::OutPoint>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<DetailedCell>> {
        let mut w = self
            .pool
            .wrapper()
            .le("block_number", tip_block_number)
            .and()
            .push_sql("(")
            .gt("consumed_block_number", tip_block_number)
            .or()
            .is_null("consumed_block_number")
            .push_sql(")")
            .and()
            .in_array("lock_hash", &lock_hashes);
        if !type_hashes.is_empty() {
            w = w.and().in_array("type_hash", &type_hashes);
        }
        if let Some(out_point) = out_point {
            let tx_hash: H256 = out_point.tx_hash().unpack();
            let output_index: u32 = out_point.index().unpack();
            w = w
                .and()
                .eq("tx_hash", to_rb_bytes(&tx_hash.0))
                .and()
                .eq("output_index", output_index);
        }
        let cells: Page<CellTable> = self
            .pool
            .fetch_page_by_wrapper(w, &PageRequest::from(pagination.clone()))
            .await?;
        let next_cursor = build_next_cursor!(cells, pagination);
        let res = cells.records.into_iter().map(Into::into).collect();
        Ok(to_pagination_response(
            res,
            next_cursor,
            if pagination.return_count {
                Some(cells.total)
            } else {
                None
            },
        ))
    }

    async fn query_tip_block(&self) -> Result<AnyRow> {
        let query = SQLXPool::new_query(
            r#"
            SELECT * FROM mercury_block 
            ORDER BY block_number
            DESC
            "#,
        );
        self.sqlx_pool.fetch_one(query).await
    }

    async fn query_block_by_hash(&self, block_hash: &[u8]) -> Result<AnyRow> {
        let query = SQLXPool::new_query(
            r#"
            SELECT * FROM mercury_block
            WHERE block_hash = $1
            "#,
        )
        .bind(block_hash);
        self.sqlx_pool.fetch_one(query).await
    }

    pub(crate) async fn query_block_by_number(&self, block_number: i64) -> Result<AnyRow> {
        let query = SQLXPool::new_query(
            r#"
            SELECT * FROM mercury_block
            WHERE block_number = $1
            "#,
        )
        .bind(block_number);
        self.sqlx_pool.fetch_one(query).await
    }

    async fn query_tip_simple_block(&self) -> Result<(H256, BlockNumber, H256, u64)> {
        let query = SQLXPool::new_query(
            r#"
            SELECT block_hash, block_number, parent_hash, block_timestamp 
            FROM mercury_block
            ORDER BY block_number
            DESC
            "#,
        );
        self.sqlx_pool.fetch_one(query).await.map(to_simple_block)
    }

    async fn query_simple_block_by_hash(
        &self,
        block_hash: &[u8],
    ) -> Result<(H256, BlockNumber, H256, u64)> {
        let query = SQLXPool::new_query(
            r#"
            SELECT block_hash, block_number, parent_hash, block_timestamp 
            FROM mercury_block
            WHERE block_hash = $1
            "#,
        )
        .bind(block_hash);
        self.sqlx_pool.fetch_one(query).await.map(to_simple_block)
    }

    async fn query_simple_block_by_number(
        &self,
        block_number: i64,
    ) -> Result<(H256, BlockNumber, H256, u64)> {
        let query = SQLXPool::new_query(
            r#"
            SELECT block_hash, block_number, parent_hash, block_timestamp 
            FROM mercury_block
            WHERE block_number = $1
            "#,
        )
        .bind(block_number);
        self.sqlx_pool.fetch_one(query).await.map(to_simple_block)
    }

    pub(crate) async fn query_indexer_cells(
        &self,
        lock_hashes: Vec<H256>,
        type_hashes: Vec<H256>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<IndexerCellTable>> {
        let mut w = self.pool.wrapper();

        if let Some(range) = block_range {
            w = w.between("block_number", range.min(), range.max());
        }

        if !lock_hashes.is_empty() {
            let lock_hashes = lock_hashes
                .iter()
                .map(|hash| to_rb_bytes(&hash.0))
                .collect::<Vec<_>>();
            w = w.and().r#in("lock_hash", &lock_hashes);
        }

        if !type_hashes.is_empty() {
            let type_hashes = type_hashes
                .iter()
                .map(|hash| to_rb_bytes(&hash.0))
                .collect::<Vec<_>>();
            w = w.and().r#in("type_hash", &type_hashes);
        }

        let res: Page<IndexerCellTable> = self
            .pool
            .fetch_page_by_wrapper(w, &PageRequest::from(pagination.clone()))
            .await?;
        let next_cursor = build_next_cursor!(res, pagination);

        Ok(to_pagination_response(
            res.records,
            next_cursor,
            if pagination.return_count {
                Some(res.total)
            } else {
                None
            },
        ))
    }

    pub(crate) async fn query_transactions_by_block_hash(
        &self,
        block_hash: &[u8],
    ) -> Result<Vec<AnyRow>> {
        let query = SQLXPool::new_query(
            r#"
            SELECT * FROM mercury_transaction
            WHERE block_hash = $1
            ORDER BY tx_index
            ASC
            "#,
        )
        .bind(block_hash);
        self.sqlx_pool.fetch_all(query).await
    }

    pub(crate) async fn query_transaction_hashes_by_block_hash(
        &self,
        block_hash: &[u8],
    ) -> Result<Vec<H256>> {
        let query = SQLXPool::new_query(
            r#"
            SELECT tx_hash FROM mercury_transaction
            WHERE block_hash = $1
            ORDER BY tx_index
            ASC
            "#,
        )
        .bind(block_hash);
        self.sqlx_pool.fetch_all(query).await.map(|tx| {
            tx.into_iter()
                .map(|tx| bytes_to_h256(&tx.get::<Vec<u8>, _>("tx_hash")))
                .collect()
        })
    }

    #[tracing_async]
    pub(crate) async fn query_transactions(
        &self,
        _ctx: Context,
        tx_hashes: Vec<Vec<u8>>,
        block_range: Option<Range>,
        pagination: PaginationRequest,
    ) -> Result<PaginationResponse<AnyRow>> {
        if tx_hashes.is_empty() && block_range.is_none() && pagination.limit == Some(0) {
            return Err(DBError::InvalidParameter(
                "no valid parameter to query transactions".to_owned(),
            )
            .into());
        }

        // build query str
        let mut query_builder = SqlBuilder::select_from("mercury_transaction");
        let mut query = query_builder.field("*");
        if !tx_hashes.is_empty() {
            query = query.and_where_in("tx_hash", &sqlx_param_placeholders(1..tx_hashes.len())?);
        }
        if let Some(ref range) = block_range {
            query = query.and_where_between(
                "block_number",
                range.from.min(i32::MAX as u64),
                range.to.min(i32::MAX as u64),
            );
        }
        let (sql, sql_for_total) = build_query_page(query, &pagination)?;

        // bind
        let bind = |sql| {
            let mut query = SQLXPool::new_query(sql);
            for hash in &tx_hashes {
                query = query.bind(hash);
            }
            query
        };
        let query = bind(&sql);
        let query_total = bind(&sql_for_total);

        // fetch
        let res = self.sqlx_pool.fetch(query).await?;
        let total = self
            .fetch_total(pagination.return_count, query_total)
            .await?;
        let next_cursor = generate_next_cursor(pagination.limit.unwrap_or(u16::MAX), &res, total);
        Ok(to_pagination_response(res, next_cursor, total))
    }

    async fn query_txs_output_cells(&self, tx_hashes: &[RbBytes]) -> Result<Vec<CellTable>> {
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }

        let w = self
            .pool
            .wrapper()
            .r#in("tx_hash", tx_hashes)
            .order_by(true, &["output_index"]);
        let cells: Vec<CellTable> = self.pool.fetch_list_by_wrapper(w).await?;
        Ok(cells)
    }

    async fn query_txs_input_cells(&self, tx_hashes: &[RbBytes]) -> Result<Vec<CellTable>> {
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }

        let w = self
            .pool
            .wrapper()
            .r#in("consumed_tx_hash", tx_hashes)
            .order_by(true, &["input_index"]);
        let cells: Vec<CellTable> = self.pool.fetch_list_by_wrapper(w).await?;
        Ok(cells)
    }

    pub(crate) async fn query_registered_address(
        &self,
        lock_hash: RbBytes,
    ) -> Result<Option<RegisteredAddressTable>> {
        let address = self.pool.fetch_by_column("lock_hash", &lock_hash).await?;
        Ok(address)
    }

    async fn fetch_total<'a, T>(
        &self,
        return_count: bool,
        query_number: Query<'a, Any, T>,
    ) -> Result<Option<u64>>
    where
        T: Send + IntoArguments<'a, Any> + 'a,
    {
        if return_count {
            let count = self
                .sqlx_pool
                .fetch_one(query_number)
                .await?
                .get::<i64, _>("total") as u64;
            Ok(Some(count))
        } else {
            Ok(None)
        }
    }
}

fn build_block_view(
    header: HeaderView,
    uncles: Vec<UncleBlockView>,
    txs: Vec<TransactionView>,
    proposals: packed::ProposalShortIdVec,
) -> BlockView {
    BlockBuilder::default()
        .header(header)
        .uncles(uncles)
        .transactions(txs)
        .proposals(proposals)
        .build()
}

fn build_header_view(block: &AnyRow) -> HeaderView {
    let epoch = if block.get::<i32, _>("block_number") == 0 {
        0u64.pack()
    } else {
        EpochNumberWithFraction::new(
            block.get::<i32, _>("epoch_number") as u64,
            block.get::<i32, _>("epoch_index") as u64,
            block.get::<i32, _>("epoch_length") as u64,
        )
        .full_value()
        .pack()
    };
    HeaderBuilder::default()
        .number((block.get::<i32, _>("block_number") as u64).pack())
        .parent_hash(packed::Byte32::new(to_fixed_array(
            &block.get::<Vec<u8>, _>("parent_hash"),
        )))
        .compact_target((block.get::<i32, _>("compact_target") as u32).pack())
        .nonce(utils::decode_nonce(&block.get::<Vec<u8>, _>("nonce")).pack())
        .timestamp((block.get::<i64, _>("block_timestamp") as u64).pack())
        .version((block.get::<i16, _>("version") as u32).pack())
        .epoch(epoch)
        .dao(packed::Byte32::new(to_fixed_array(
            &block.get::<Vec<u8>, _>("dao")[0..32],
        )))
        .transactions_root(packed::Byte32::new(to_fixed_array(
            &block.get::<Vec<u8>, _>("transactions_root")[0..32],
        )))
        .proposals_hash(packed::Byte32::new(to_fixed_array(
            &block.get::<Vec<u8>, _>("proposals_hash")[0..32],
        )))
        .extra_hash(packed::Byte32::new(to_fixed_array(
            &block.get::<Vec<u8>, _>("uncles_hash")[0..32],
        )))
        .build()
}

fn build_witnesses(input: Vec<u8>) -> packed::BytesVec {
    packed::BytesVec::new_unchecked(Bytes::from(input))
}

fn build_header_deps(input: Vec<u8>) -> packed::Byte32Vec {
    packed::Byte32Vec::new_unchecked(Bytes::from(input))
}

fn build_cell_deps(input: Vec<u8>) -> packed::CellDepVec {
    packed::CellDepVec::new_unchecked(Bytes::from(input))
}

fn build_proposals(input: Vec<u8>) -> packed::ProposalShortIdVec {
    packed::ProposalShortIdVec::new_unchecked(Bytes::from(input))
}

fn build_cell_base_input(block_number: u64) -> packed::CellInput {
    let out_point = packed::OutPointBuilder::default()
        .tx_hash(packed::Byte32::default())
        .index(u32::MAX.pack())
        .build();
    packed::CellInputBuilder::default()
        .since(block_number.pack())
        .previous_output(out_point)
        .build()
}

fn build_cell_inputs(input_cells: Iter<CellTable>) -> Vec<packed::CellInput> {
    input_cells
        .map(|cell| {
            let out_point = packed::OutPointBuilder::default()
                .tx_hash(
                    packed::Byte32::from_slice(&cell.tx_hash.inner)
                        .expect("impossible: fail to pack since"),
                )
                .index((cell.output_index as u32).pack())
                .build();

            packed::CellInputBuilder::default()
                .since(decode_since(&cell.since.inner).pack())
                .previous_output(out_point)
                .build()
        })
        .collect()
}

fn build_cell_outputs(cells: &[CellTable]) -> (Vec<packed::CellOutput>, Vec<packed::Bytes>) {
    (
        cells
            .iter()
            .map(|cell| {
                let lock_script: packed::Script = cell.to_lock_script_table().into();
                let type_script_opt = build_script_opt(if cell.has_type_script() {
                    Some(cell.to_type_script_table())
                } else {
                    None
                });
                packed::CellOutputBuilder::default()
                    .capacity(cell.capacity.pack())
                    .lock(lock_script)
                    .type_(type_script_opt)
                    .build()
            })
            .collect(),
        cells.iter().map(|cell| cell.data.inner.pack()).collect(),
    )
}

fn build_script_opt(script_opt: Option<ScriptTable>) -> packed::ScriptOpt {
    let script_opt = script_opt.map(|script| script.into());
    packed::ScriptOptBuilder::default().set(script_opt).build()
}

fn build_transaction_view(
    version: u32,
    witnesses: packed::BytesVec,
    inputs: Vec<packed::CellInput>,
    outputs: Vec<packed::CellOutput>,
    outputs_data: Vec<packed::Bytes>,
    cell_deps: packed::CellDepVec,
    header_deps: packed::Byte32Vec,
) -> TransactionView {
    TransactionBuilder::default()
        .version(version.pack())
        .witnesses(witnesses)
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data)
        .cell_deps(cell_deps)
        .header_deps(header_deps)
        .build()
}

pub fn to_pagination_response<T>(
    records: Vec<T>,
    next: Option<u64>,
    total: Option<u64>,
) -> PaginationResponse<T> {
    PaginationResponse {
        response: records,
        next_cursor: next.map(Into::into),
        count: total.map(Into::into),
    }
}

pub fn rb_bytes_to_h256(input: &RbBytes) -> H256 {
    H256::from_slice(&input.inner[0..32]).expect("rb bytes to h256")
}

pub fn bytes_to_h256(input: &[u8]) -> H256 {
    H256::from_slice(&input[0..32]).expect("bytes to h256")
}

fn to_simple_block(block: AnyRow) -> (H256, BlockNumber, H256, u64) {
    (
        bytes_to_h256(&block.get::<Vec<u8>, _>("block_hash")),
        block
            .get::<i32, _>("block_number")
            .try_into()
            .expect("i32 to u64"),
        bytes_to_h256(&block.get::<Vec<u8>, _>("parent_hash")),
        block
            .get::<i64, _>("block_timestamp")
            .try_into()
            .expect("i64 to u64"),
    )
}

fn sqlx_param_placeholders(range: std::ops::Range<usize>) -> Result<Vec<String>> {
    if range.start == 0 {
        return Err(DBError::InvalidParameter(
            "no valid parameter to query transactions".to_owned(),
        )
        .into());
    }
    Ok((1..=range.end)
        .map(|i| format!("${}", i))
        .collect::<Vec<String>>())
}

fn generate_next_cursor(limit: u16, records: &[AnyRow], total: Option<u64>) -> Option<u64> {
    let mut next_cursor = None;
    if records.len() == limit as usize {
        let last = records.last().unwrap().get::<i64, _>("id") as u64;
        if let Some(total) = total {
            if total > limit as u64 {
                next_cursor = Some(last)
            }
        } else {
            next_cursor = Some(last);
        }
    }
    next_cursor
}

fn build_query_page(
    mut query: &mut SqlBuilder,
    pagination: &PaginationRequest,
) -> Result<(String, String)> {
    if let Some(id) = pagination.cursor {
        let id = i64::try_from(id).unwrap_or(i64::MAX);
        match pagination.order {
            Order::Asc => query = query.and_where_ge("id", id),
            Order::Desc => query = query.and_where_le("id", id),
        }
    }
    let sql_sub_query = query.subquery()?;
    match pagination.order {
        Order::Asc => query = query.order_by("id", false),
        Order::Desc => query = query.order_by("id", true),
    }
    if let Some(limit) = pagination.limit {
        let limit = i64::try_from(limit)?;
        query = query.limit(limit);
    }

    let query = query.sql()?.trim_end_matches(';').to_string();
    let sub_query_for_count = SqlBuilder::select_from(format!("{} res", sql_sub_query))
        .field("COUNT(*) AS total")
        .sql()?
        .trim_end_matches(';')
        .to_string();

    Ok((query, sub_query_for_count))
}

fn build_detailed_cell(row: AnyRow) -> Result<DetailedCell> {
    let lock_script = packed::ScriptBuilder::default()
        .code_hash(to_fixed_array::<32>(&row.get::<Vec<u8>, _>("lock_code_hash")[0..32]).pack())
        .args(row.get::<Vec<u8>, _>("lock_args").pack())
        .hash_type(packed::Byte::new(
            row.get::<i16, _>("lock_script_type").try_into()?,
        ))
        .build();
    let type_script = if row.get::<Vec<u8>, _>("type_hash") == H256::default().as_bytes() {
        None
    } else {
        Some(
            packed::ScriptBuilder::default()
                .code_hash(H256::from_slice(&row.get::<Vec<u8>, _>("type_code_hash"))?.pack())
                .args(row.get::<Vec<u8>, _>("type_args").pack())
                .hash_type(packed::Byte::new(
                    row.get::<i16, _>("type_script_type").try_into()?,
                ))
                .build(),
        )
    };

    let convert_hash = |hash: Option<Vec<u8>>| -> Option<H256> {
        if let Some(hash) = hash {
            if hash.is_empty() {
                None
            } else {
                Some(H256::from_slice(&hash).expect("convert hash"))
            }
        } else {
            None
        }
    };

    let convert_since = |b: Option<Vec<u8>>| -> Option<u64> {
        if let Some(b) = b {
            if b.is_empty() {
                None
            } else {
                Some(u64::from_be_bytes(to_fixed_array::<8>(&b)))
            }
        } else {
            None
        }
    };

    let cell = DetailedCell {
        epoch_number: EpochNumberWithFraction::new_unchecked(
            row.get::<i32, _>("epoch_number").try_into()?,
            row.get::<i32, _>("epoch_index").try_into()?,
            row.get::<i32, _>("epoch_length").try_into()?,
        )
        .full_value(),
        block_number: row.get::<i32, _>("block_number").try_into()?,
        block_hash: H256::from_slice(&row.get::<Vec<u8>, _>("block_hash")[0..32])?,
        tx_index: row.get::<i32, _>("tx_index").try_into()?,
        out_point: packed::OutPointBuilder::default()
            .tx_hash(to_fixed_array::<32>(&row.get::<Vec<u8>, _>("tx_hash")).pack())
            .index(u32::try_from(row.get::<i32, _>("output_index"))?.pack())
            .build(),
        cell_output: packed::CellOutputBuilder::default()
            .lock(lock_script)
            .type_(type_script.pack())
            .capacity(u64::try_from(row.get::<i64, _>("capacity"))?.pack())
            .build(),
        cell_data: row.get::<Vec<u8>, _>("data").clone().into(),

        // The following fields are in the mercury_cell table, but not in the mercury_live_cell table
        consumed_block_hash: convert_hash(row.try_get::<Vec<u8>, _>("consumed_block_hash").ok()),
        consumed_block_number: row
            .try_get::<i64, _>("consumed_block_number")
            .map(|block_number| block_number as u64)
            .ok(),
        consumed_tx_hash: convert_hash(row.try_get::<Vec<u8>, _>("consumed_tx_hash").ok()),
        consumed_tx_index: row
            .try_get::<i32, _>("consumed_tx_index")
            .map(|block_number| block_number as u32)
            .ok(),
        consumed_input_index: row
            .try_get::<i32, _>("input_index")
            .map(|block_number| block_number as u32)
            .ok(),
        since: convert_since(row.try_get::<Vec<u8>, _>("since").ok()),
    };
    Ok(cell)
}
