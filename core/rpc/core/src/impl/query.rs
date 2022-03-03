use crate::r#impl::utils;
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use common::{Context, Order, PaginationRequest, PaginationResponse, Range};
use common_logger::tracing_async;
use core_ckb_client::CkbRpc;
use core_rpc_types::lazy::{CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER};
use core_rpc_types::{
    indexer, AssetInfo, Balance, BlockInfo, BurnInfo, GetBalancePayload, GetBalanceResponse,
    GetBlockInfoPayload, GetSpentTransactionPayload, GetTransactionInfoResponse, IOType, Item,
    Ownership, QueryTransactionsPayload, Record, StructureType, SyncProgress, SyncState,
    TransactionInfo, TransactionStatus, TxView,
};
use core_storage::{DBInfo, Storage, TransactionWrapper};

use ckb_jsonrpc_types::{self, Capacity, Script, Uint64};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self, ctx: Context) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info(ctx)
            .map_err(|error| CoreError::DBError(error.to_string()).into())
    }

    #[tracing_async]
    pub(crate) async fn inner_get_balance(
        &self,
        ctx: Context,
        payload: GetBalancePayload,
    ) -> InnerResult<GetBalanceResponse> {
        let item: Item = payload.item.clone().try_into()?;
        let tip_epoch_number = if let Some(tip_block_number) = payload.tip_block_number {
            Some(
                self.get_epoch_by_number(ctx.clone(), tip_block_number)
                    .await?,
            )
        } else {
            None
        };

        let ckb_asset_info = AssetInfo::new_ckb();
        let asset_infos = if payload.asset_infos.contains(&ckb_asset_info) {
            // to get statistics on free, occupied, freezed and claimable
            // need all kind of cells
            HashSet::new()
        } else {
            payload.asset_infos.clone()
        };
        let live_cells = self
            .get_live_cells_by_item(
                ctx.clone(),
                item.clone(),
                asset_infos,
                payload.tip_block_number,
                tip_epoch_number.clone(),
                None,
                None,
                &mut PaginationRequest::default(),
            )
            .await?;

        let mut balances_map: HashMap<(Ownership, AssetInfo), Balance> = HashMap::new();
        
        for cell in live_cells {
            let records = self
                .to_record(
                    ctx.clone(),
                    &cell,
                    IOType::Output,
                    payload.tip_block_number,
                    tip_epoch_number.clone(),
                )
                .await?;

            let records: Vec<Record> = records
                .into_iter()
                .filter(|record| {
                    payload.asset_infos.contains(&record.asset_info)
                        || payload.asset_infos.is_empty()
                })
                .collect();
            self.accumulate_balance_from_records(
                ctx.clone(),
                &mut balances_map,
                &records,
                tip_epoch_number.clone(),
            )
            .await?;
        }

        let balances = balances_map
            .into_iter()
            .map(|(_, balance)| balance)
            .collect();

        Ok(GetBalanceResponse {
            balances,
            tip_block_number: payload
                .tip_block_number
                .unwrap_or(**CURRENT_BLOCK_NUMBER.load()),
        })
    }

    #[tracing_async]
    pub(crate) async fn inner_get_block_info(
        &self,
        ctx: Context,
        payload: GetBlockInfoPayload,
    ) -> InnerResult<BlockInfo> {
        let block_info = self
            .storage
            .get_simple_block(ctx.clone(), payload.block_hash, payload.block_number)
            .await;
        let block_info = match block_info {
            Ok(block_info) => block_info,
            Err(error) => return Err(CoreError::DBError(error.to_string()).into()),
        };

        let mut transactions = vec![];
        for tx_hash in block_info.transactions {
            let tx_info = self
                .inner_get_transaction_info(ctx.clone(), tx_hash)
                .await
                .map(|res| res.transaction.expect("impossible: cannot find the tx"))?;
            transactions.push(tx_info);
        }

        Ok(BlockInfo {
            block_number: block_info.block_number,
            block_hash: block_info.block_hash,
            parent_hash: block_info.parent_hash,
            timestamp: block_info.timestamp,
            transactions,
        })
    }

    #[tracing_async]
    pub(crate) async fn inner_query_transactions(
        &self,
        ctx: Context,
        payload: QueryTransactionsPayload,
    ) -> InnerResult<PaginationResponse<TxView>> {
        let pagination_ret = self
            .get_transactions_by_item(
                ctx.clone(),
                payload.item.try_into()?,
                payload.asset_infos,
                payload.extra,
                payload.block_range,
                payload.pagination,
            )
            .await?;
        match &payload.structure_type {
            StructureType::Native => Ok(PaginationResponse {
                response: pagination_ret
                    .response
                    .into_iter()
                    .map(|tx_wrapper| TxView::TransactionWithRichStatus(tx_wrapper.into()))
                    .collect(),
                next_cursor: pagination_ret.next_cursor,
                count: pagination_ret.count,
            }),

            StructureType::DoubleEntry => {
                let mut tx_infos = vec![];
                for tx_view in pagination_ret.response.into_iter() {
                    let tx_info = TxView::TransactionInfo(
                        self.query_transaction_info(ctx.clone(), &tx_view).await?,
                    );
                    tx_infos.push(tx_info);
                }
                Ok(PaginationResponse {
                    response: tx_infos,
                    next_cursor: pagination_ret.next_cursor,
                    count: pagination_ret.count,
                })
            }
        }
    }

    #[tracing_async]
    pub(crate) async fn inner_get_tip(&self, ctx: Context) -> InnerResult<Option<indexer::Tip>> {
        let block = self
            .storage
            .get_tip(ctx)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        if let Some((block_number, block_hash)) = block {
            Ok(Some(indexer::Tip {
                block_number: block_number.into(),
                block_hash,
            }))
        } else {
            Ok(None)
        }
    }

    #[tracing_async]
    pub(crate) async fn inner_get_cells(
        &self,
        ctx: Context,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
    ) -> InnerResult<indexer::PaginationResponse<indexer::Cell>> {
        let pagination = {
            let order: common::Order = order.into();
            PaginationRequest::new(after_cursor, order, Some(limit.into()), None, false)
        };
        let db_response = self
            .get_live_cells_by_search_key(ctx.clone(), search_key, pagination)
            .await?;

        let objects: Vec<indexer::Cell> = db_response
            .response
            .into_iter()
            .map(|cell| cell.into())
            .collect();
        Ok(indexer::PaginationResponse {
            objects,
            last_cursor: db_response.next_cursor,
        })
    }

    #[tracing_async]
    pub(crate) async fn inner_get_spent_transaction(
        &self,
        ctx: Context,
        payload: GetSpentTransactionPayload,
    ) -> InnerResult<TxView> {
        let tx_hash = self
            .storage
            .get_spent_transaction_hash(ctx.clone(), payload.outpoint.into())
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        let tx_hash = match tx_hash {
            Some(tx_hash) => tx_hash,
            None => return Err(CoreError::CannotFindSpentTransaction.into()),
        };

        match &payload.structure_type {
            StructureType::Native => {
                let tx = self
                    .inner_get_transaction_with_status(ctx.clone(), tx_hash)
                    .await?;
                Ok(TxView::TransactionWithRichStatus(tx.into()))
            }
            StructureType::DoubleEntry => self
                .inner_get_transaction_info(ctx.clone(), tx_hash)
                .await
                .map(|res| {
                    TxView::TransactionInfo(
                        res.transaction.expect("impossible: cannot find the tx"),
                    )
                }),
        }
    }

    #[tracing_async]
    pub(crate) async fn inner_get_cells_capacity(
        &self,
        ctx: Context,
        payload: indexer::SearchKey,
    ) -> InnerResult<indexer::CellsCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, false);
        let db_response = self
            .get_live_cells_by_search_key(ctx.clone(), payload, pagination)
            .await?;
        let capacity: u64 = db_response
            .response
            .into_iter()
            .map(|cell| -> u64 { cell.cell_output.capacity().unpack() })
            .sum();

        let block = self
            .storage
            .get_tip(ctx)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        let (block_number, block_hash) =
            block.ok_or_else(|| CoreError::DBError(String::from("fail to get tip block")))?;

        Ok(indexer::CellsCapacity {
            capacity: capacity.into(),
            block_hash,
            block_number: block_number.into(),
        })
    }

    #[tracing_async]
    pub(crate) async fn inner_get_transaction(
        &self,
        ctx: Context,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
    ) -> InnerResult<indexer::PaginationResponse<indexer::Transaction>> {
        let pagination = {
            let order: common::Order = order.into();
            PaginationRequest::new(after_cursor, order, Some(limit.into()), None, false)
        };

        let script = search_key.script;
        let (the_other_script, block_range) = if let Some(filter) = search_key.filter {
            (filter.script, filter.block_range)
        } else {
            (None, None)
        };
        let (lock_script, type_script) = match search_key.script_type {
            indexer::ScriptType::Lock => (Some(script), the_other_script),
            indexer::ScriptType::Type => (the_other_script, Some(script)),
        };
        let lock_script: Option<packed::Script> = lock_script.map(Into::into);
        let type_script: Option<packed::Script> = type_script.map(Into::into);
        let block_range = block_range.map(|range| Range::new(range[0].into(), range[1].into()));

        let db_response = self
            .storage
            .get_indexer_transactions(
                ctx.clone(),
                lock_script.map_or_else(Vec::new, |s| vec![s.calc_script_hash().unpack()]),
                type_script.map_or_else(Vec::new, |s| vec![s.calc_script_hash().unpack()]),
                block_range,
                pagination,
            )
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;

        let mut objects = Vec::new();
        for cell in db_response.response.iter() {
            let object = indexer::Transaction {
                tx_hash: H256::from_slice(&cell.tx_hash.inner[0..32]).unwrap(),
                block_number: cell.block_number.into(),
                tx_index: cell.tx_index.into(),
                io_index: cell.io_index.into(),
                io_type: if cell.io_type == 0 {
                    indexer::IOType::Input
                } else {
                    indexer::IOType::Output
                },
            };
            objects.push(object);
        }

        Ok(indexer::PaginationResponse {
            objects,
            last_cursor: db_response.next_cursor,
        })
    }

    #[tracing_async]
    pub(crate) async fn inner_get_live_cells_by_lock_hash(
        &self,
        ctx: Context,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> InnerResult<Vec<indexer::LiveCell>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            let page: u64 = page.into();
            let per_page: u64 = per_page.into();
            if per_page > 50 {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                ))
                .into());
            }
            let skip = page * per_page;
            let limit = per_page;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let cells = self
            .storage
            .get_live_cells(
                ctx.clone(),
                None,
                vec![lock_hash],
                vec![],
                None,
                None,
                None,
                pagination,
            )
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        let res: Vec<indexer::LiveCell> = cells
            .response
            .into_iter()
            .map(|cell| {
                let index: u32 = cell.out_point.index().unpack();
                let tranaction_point = indexer::TransactionPoint {
                    block_number: cell.block_number.into(),
                    tx_hash: cell.out_point.tx_hash().unpack(),
                    index: index.into(),
                };
                indexer::LiveCell {
                    created_by: tranaction_point,
                    cell_output: cell.cell_output.into(),
                    output_data_len: (cell.cell_data.len() as u64).into(),
                    cellbase: cell.tx_index == 0,
                }
            })
            .collect();
        Ok(res)
    }

    #[tracing_async]
    pub(crate) async fn inner_get_capacity_by_lock_hash(
        &self,
        ctx: Context,
        lock_hash: H256,
    ) -> InnerResult<indexer::LockHashCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, true);
        let db_response = self
            .storage
            .get_cells(ctx.clone(), None, vec![lock_hash], vec![], None, pagination)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;

        let cells_count = db_response
            .count
            .ok_or_else(|| CoreError::DBError(String::from("fail to get cells count")))?;
        let capacity: u64 = db_response
            .response
            .into_iter()
            .map(|cell| -> u64 { cell.cell_output.capacity().unpack() })
            .sum();

        let block = self
            .storage
            .get_tip(ctx.clone())
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        let (block_number, _) =
            block.ok_or_else(|| CoreError::DBError(String::from("fail to get tip block")))?;

        Ok(indexer::LockHashCapacity {
            capacity: Capacity::from(capacity),
            cells_count: cells_count.into(),
            block_number: block_number.into(),
        })
    }

    #[allow(clippy::unnecessary_unwrap)]
    #[tracing_async]
    pub(crate) async fn inner_get_transactions_by_lock_hash(
        &self,
        ctx: Context,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> InnerResult<Vec<indexer::CellTransaction>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            let page: u64 = page.into();
            let per_page: u64 = per_page.into();
            if per_page > 50 {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                ))
                .into());
            }
            let skip = page * per_page;
            let limit = per_page;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let db_response = self
            .storage
            .get_cells(ctx.clone(), None, vec![lock_hash], vec![], None, pagination)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;

        let mut cell_txs: Vec<indexer::CellTransaction> = vec![];
        for cell in db_response.response.iter() {
            let out_cell = cell.clone();
            let created_by = indexer::TransactionPoint {
                block_number: out_cell.block_number.into(),
                tx_hash: out_cell.out_point.tx_hash().unpack(),
                index: out_cell.out_point.index().unpack(),
            };
            let out_point = cell.out_point.clone();
            let consumed_by = {
                let consume_info = self
                    .storage
                    .get_cells(
                        ctx.clone(),
                        Some(out_point),
                        vec![],
                        vec![],
                        None,
                        Default::default(),
                    )
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?
                    .response;
                if let Some(cell) = consume_info.get(0).cloned() {
                    let (block_number, tx_hash, index) = (
                        cell.consumed_block_number,
                        cell.consumed_tx_hash,
                        cell.consumed_tx_index,
                    );
                    if block_number.is_none() || tx_hash.is_none() || index.is_none() {
                        None
                    } else {
                        Some(indexer::TransactionPoint {
                            block_number: block_number.unwrap().into(),
                            tx_hash: tx_hash.unwrap(),
                            index: index.unwrap().into(),
                        })
                    }
                } else {
                    None
                }
            };

            cell_txs.push(indexer::CellTransaction {
                created_by,
                consumed_by,
            })
        }

        Ok(cell_txs)
    }

    #[tracing_async]
    pub(crate) async fn inner_get_transaction_with_status(
        &self,
        ctx: Context,
        tx_hash: H256,
    ) -> InnerResult<TransactionWrapper> {
        let tx_wrapper = self
            .storage
            .get_transactions_by_hashes(
                ctx.clone(),
                vec![tx_hash.clone()],
                None,
                Default::default(),
            )
            .await;
        let tx_wrapper = match tx_wrapper {
            Ok(tx_wrapper) => tx_wrapper,
            Err(error) => return Err(CoreError::DBError(error.to_string()).into()),
        };
        let tx_wrapper = match tx_wrapper.response.get(0).cloned() {
            Some(tx_wrapper) => tx_wrapper,
            None => return Err(CoreError::CannotFindTransactionByHash.into()),
        };

        Ok(tx_wrapper)
    }

    #[tracing_async]
    pub(crate) async fn inner_get_transaction_info(
        &self,
        ctx: Context,
        tx_hash: H256,
    ) -> InnerResult<GetTransactionInfoResponse> {
        let tx = self
            .inner_get_transaction_with_status(ctx.clone(), tx_hash)
            .await?;
        let transaction = self.query_transaction_info(ctx.clone(), &tx).await?;

        Ok(GetTransactionInfoResponse {
            transaction: Some(transaction),
            status: TransactionStatus::committed,
            reject_reason: None,
        })
    }

    #[tracing_async]
    async fn query_transaction_info(
        &self,
        ctx: Context,
        tx_wrapper: &TransactionWrapper,
    ) -> InnerResult<TransactionInfo> {
        let mut records: Vec<Record> = vec![];

        let tip_block_number = **CURRENT_BLOCK_NUMBER.load();
        let tip_epoch_number = (**CURRENT_EPOCH_NUMBER.load()).clone();
        let tx_hash = tx_wrapper
            .transaction_with_status
            .transaction
            .clone()
            .expect("impossible: get transaction fail")
            .hash;

        for input_cell in &tx_wrapper.input_cells {
            let mut input_records = self
                .to_record(
                    ctx.clone(),
                    input_cell,
                    IOType::Input,
                    Some(tip_block_number),
                    Some(tip_epoch_number.clone()),
                )
                .await?;
            records.append(&mut input_records);
        }

        for output_cell in &tx_wrapper.output_cells {
            let mut output_records = self
                .to_record(
                    ctx.clone(),
                    output_cell,
                    IOType::Output,
                    Some(tip_block_number),
                    Some(tip_epoch_number.clone()),
                )
                .await?;
            records.append(&mut output_records);
        }

        let mut map: HashMap<H256, BigInt> = HashMap::new();
        for record in &records {
            let entry = map
                .entry(record.asset_info.udt_hash.clone())
                .or_insert_with(BigInt::zero);
            *entry += record
                .amount
                .parse::<BigInt>()
                .expect("impossible: parse big int fail");
        }

        let fee = map
            .get(&H256::default())
            .map(|amount| -amount)
            .unwrap_or_else(BigInt::zero)
            .to_i64()
            .expect("impossible: get fee fail");

        // tips: according to the calculation rule, coinbase and dao claim transaction will get negative fee which is unreasonable.
        let fee = if fee < 0 { 0 } else { fee as u64 };

        Ok(TransactionInfo {
            tx_hash,
            records,
            fee,
            burn: map
                .iter()
                .filter(|(udt_hash, _)| **udt_hash != H256::default())
                .map(|(udt_hash, amount)| BurnInfo {
                    udt_hash: udt_hash.to_owned(),
                    amount: (-amount).to_string(),
                })
                .collect(),
            timestamp: tx_wrapper.timestamp,
        })
    }

    #[tracing_async]
    async fn get_live_cells_by_search_key(
        &self,
        ctx: Context,
        search_key: indexer::SearchKey,
        pagination: PaginationRequest,
    ) -> InnerResult<PaginationResponse<common::DetailedCell>> {
        let script = search_key.script;
        let (the_other_script, output_data_len_range, output_capacity_range, block_range) =
            if let Some(filter) = search_key.filter {
                (
                    filter.script,
                    filter.output_data_len_range,
                    filter.output_capacity_range,
                    filter.block_range,
                )
            } else {
                (None, None, None, None)
            };
        let (lock_script, type_script) = match search_key.script_type {
            indexer::ScriptType::Lock => (Some(script), the_other_script),
            indexer::ScriptType::Type => (the_other_script, Some(script)),
        };
        let cal_script_hash = |script: Option<Script>| -> Vec<H256> {
            if let Some(script) = script {
                let script: packed::Script = script.into();
                vec![H256::from_slice(&script.calc_script_hash().as_bytes()).unwrap()]
            } else {
                vec![]
            }
        };
        let lock_hashes = cal_script_hash(lock_script);
        let type_hashes = cal_script_hash(type_script);
        let block_range = block_range.map(|range| Range::new(range[0].into(), range[1].into()));
        let capacity_range = output_capacity_range.map(|range| {
            let to: u64 = range[1].into();
            Range::new(range[0].into(), to.saturating_sub(1))
        });
        let data_len_range = output_data_len_range.map(|range| {
            let to: u64 = range[1].into();
            Range::new(range[0].into(), to.saturating_sub(1))
        });
        let db_response = self
            .storage
            .get_live_cells(
                ctx.clone(),
                None,
                lock_hashes,
                type_hashes,
                block_range,
                capacity_range,
                data_len_range,
                pagination,
            )
            .await;
        let db_response = db_response.map_err(|error| CoreError::DBError(error.to_string()))?;
        Ok(db_response)
    }

    pub(crate) async fn inner_get_sync_state(&self, ctx: Context) -> InnerResult<SyncState> {
        let state = (&*self.sync_state.read()).to_owned();
        match state {
            SyncState::ReadOnly => Ok(state.to_owned()),
            SyncState::ParallelFirstStage(sync_process) => {
                let current_count = self
                    .storage
                    .block_count(ctx.clone())
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?;
                let state = SyncState::ParallelFirstStage(SyncProgress::new(
                    current_count.saturating_sub(1),
                    sync_process.target,
                    utils::calculate_the_percentage(
                        current_count.saturating_sub(1),
                        sync_process.target,
                    ),
                ));
                Ok(state)
            }
            SyncState::ParallelSecondStage(_) => {
                let indexer_synced_count = self
                    .storage
                    .indexer_synced_count()
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?;
                let tip_number = self
                    .storage
                    .get_tip_number()
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?;

                let state = SyncState::ParallelSecondStage(SyncProgress::new(
                    indexer_synced_count.saturating_sub(1),
                    tip_number,
                    utils::calculate_the_percentage(
                        indexer_synced_count.saturating_sub(1),
                        tip_number,
                    ),
                ));
                Ok(state)
            }
            SyncState::Serial(_) => {
                let node_tip = self
                    .ckb_client
                    .get_tip_block_number()
                    .await
                    .map_err(|error| CoreError::CkbClientError(error.to_string()))?;
                let tip_number = self
                    .storage
                    .get_tip_number()
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?;
                let state = SyncState::Serial(SyncProgress::new(
                    tip_number,
                    node_tip,
                    utils::calculate_the_percentage(tip_number, node_tip),
                ));
                Ok(state)
            }
        }
    }
}
