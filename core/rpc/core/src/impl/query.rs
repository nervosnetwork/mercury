use crate::r#impl::utils;
use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use common::{Order, PaginationRequest, Range};
use core_ckb_client::CkbRpc;
use core_rpc_types::lazy::CURRENT_BLOCK_NUMBER;
use core_rpc_types::{
    indexer, indexer::Cell, AssetInfo, Balance, BlockInfo, BurnInfo, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, IOType, Item, PaginationResponse, QueryTransactionsPayload, Record,
    StructureType, SyncProgress, SyncState, TransactionInfo, TransactionStatus, TxView,
};
use core_storage::{DBInfo, DetailedCell, Storage, TransactionWrapper};

use ckb_jsonrpc_types::{self, Capacity, JsonBytes};
use ckb_types::{prelude::*, H256};
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::From;
use std::ops::Neg;
use std::{convert::TryInto, iter::Iterator};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info()
            .map_err(|error| CoreError::DBError(error.to_string()).into())
    }

    pub(crate) async fn inner_get_balance(
        &self,
        payload: GetBalancePayload,
    ) -> InnerResult<GetBalanceResponse> {
        let item: Item = payload.item.clone().try_into()?;
        let tip_epoch_number = if let Some(tip_block_number) = payload.tip_block_number {
            Some(self.get_epoch_by_number(tip_block_number.into()).await?)
        } else {
            None
        };

        let ckb_asset_info = AssetInfo::new_ckb();
        let asset_infos = if payload.asset_infos.contains(&ckb_asset_info) {
            // to get statistics on free, occupied, frozen
            // need all kind of cells
            HashSet::new()
        } else {
            payload.asset_infos.clone()
        };
        let live_cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_infos,
                payload.tip_block_number.map(Into::into),
                tip_epoch_number.clone(),
                HashMap::new(),
                payload.extra,
                &mut PaginationRequest::default(),
            )
            .await?;

        let mut balances_map: BTreeMap<(String, AssetInfo), Balance> = BTreeMap::new();

        for cell in live_cells {
            let records = self
                .to_record(
                    &cell,
                    IOType::Output,
                    payload.tip_block_number.map(Into::into),
                )
                .await?;
            let records: Vec<Record> = records
                .into_iter()
                .filter(|record| {
                    payload.asset_infos.contains(&record.asset_info)
                        || payload.asset_infos.is_empty()
                })
                .filter(|record| {
                    self.filter_cheque_record(record, &item, &cell, tip_epoch_number.clone())
                })
                .collect();
            self.accumulate_balance_from_records(
                &mut balances_map,
                records,
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
                .unwrap_or_else(|| (**CURRENT_BLOCK_NUMBER.load()).into()),
        })
    }

    pub(crate) async fn inner_get_block_info(
        &self,
        payload: GetBlockInfoPayload,
    ) -> InnerResult<BlockInfo> {
        let block_info = self
            .storage
            .get_simple_block(payload.block_hash, payload.block_number.map(Into::into))
            .await;
        let block_info = match block_info {
            Ok(block_info) => block_info,
            Err(error) => return Err(CoreError::DBError(error.to_string()).into()),
        };

        let mut transactions = vec![];
        for tx_hash in block_info.transactions {
            let tx_info = self
                .inner_get_transaction_info(tx_hash)
                .await
                .map(|res| res.transaction.expect("impossible: cannot find the tx"))?;
            transactions.push(tx_info);
        }

        Ok(BlockInfo {
            block_number: block_info.block_number.into(),
            block_hash: block_info.block_hash,
            parent_hash: block_info.parent_hash,
            timestamp: block_info.timestamp.into(),
            transactions,
        })
    }

    pub(crate) async fn inner_query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> InnerResult<PaginationResponse<TxView>> {
        let pagination_ret = self
            .get_transactions_by_item(
                payload.item.try_into()?,
                payload.asset_infos,
                payload.extra,
                payload.block_range.map(Into::into),
                payload.pagination.into(),
            )
            .await?;
        match &payload.structure_type {
            StructureType::Native => Ok(PaginationResponse {
                response: pagination_ret
                    .response
                    .into_iter()
                    .map(|tx_wrapper| TxView::TransactionWithRichStatus(tx_wrapper.into()))
                    .collect(),
                next_cursor: pagination_ret.next_cursor.map(Into::into),
                count: pagination_ret.count.map(Into::into),
            }),

            StructureType::DoubleEntry => {
                let mut tx_infos = vec![];
                for tx_view in pagination_ret.response.into_iter() {
                    let tx_info =
                        TxView::TransactionInfo(self.query_transaction_info(&tx_view).await?);
                    tx_infos.push(tx_info);
                }
                Ok(PaginationResponse {
                    response: tx_infos,
                    next_cursor: pagination_ret.next_cursor.map(Into::into),
                    count: pagination_ret.count.map(Into::into),
                })
            }
        }
    }

    pub(crate) async fn inner_get_tip(&self) -> InnerResult<Option<indexer::Tip>> {
        let block = self
            .storage
            .get_tip()
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

    pub(crate) async fn inner_get_cells(
        &self,
        search_key: indexer::SearchKey,
        order: Order,
        limit: u16,
        after_cursor: Option<u64>,
    ) -> InnerResult<indexer::PaginationResponse<indexer::Cell>> {
        let pagination = PaginationRequest::new(after_cursor, order, Some(limit), None, false);
        let with_data = search_key.with_data.unwrap_or(true);
        let db_response = self
            .get_live_cells_by_search_key(search_key, pagination)
            .await?;
        let objects: Vec<indexer::Cell> = db_response
            .response
            .into_iter()
            .map(|cell| Cell {
                output: cell.cell_output.into(),
                output_data: if with_data {
                    Some(JsonBytes::from_bytes(cell.cell_data))
                } else {
                    None
                },
                out_point: cell.out_point.into(),
                block_number: cell.block_number.into(),
                tx_index: cell.tx_index.into(),
            })
            .collect();
        Ok(indexer::PaginationResponse {
            objects,
            last_cursor: db_response.next_cursor.map(Into::into),
        })
    }

    pub(crate) async fn inner_get_spent_transaction(
        &self,
        payload: GetSpentTransactionPayload,
    ) -> InnerResult<TxView> {
        let tx_hash = self
            .storage
            .get_spent_transaction_hash(payload.outpoint.into())
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;
        let tx_hash = match tx_hash {
            Some(tx_hash) => tx_hash,
            None => return Err(CoreError::CannotFindSpentTransaction.into()),
        };

        match &payload.structure_type {
            StructureType::Native => {
                let tx = self.inner_get_transaction_with_status(tx_hash).await?;
                Ok(TxView::TransactionWithRichStatus(tx.into()))
            }
            StructureType::DoubleEntry => {
                self.inner_get_transaction_info(tx_hash).await.map(|res| {
                    TxView::TransactionInfo(
                        res.transaction.expect("impossible: cannot find the tx"),
                    )
                })
            }
        }
    }

    pub(crate) async fn inner_get_cells_capacity(
        &self,
        search_key: indexer::SearchKey,
    ) -> InnerResult<indexer::CellsCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, false);
        let db_response = self
            .get_live_cells_by_search_key(search_key, pagination)
            .await?;
        let capacity: u64 = db_response
            .response
            .into_iter()
            .map(|cell| -> u64 { cell.cell_output.capacity().unpack() })
            .sum();

        let block = self
            .storage
            .get_tip()
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

    pub(crate) async fn inner_get_transaction(
        &self,
        search_key: indexer::SearchKey,
        order: Order,
        limit: u16,
        after_cursor: Option<u64>,
    ) -> InnerResult<indexer::PaginationResponse<indexer::Transaction>> {
        let pagination = PaginationRequest::new(after_cursor, order, Some(limit), None, false);
        let script = search_key.script;
        let (the_other_script, block_range) = if let Some(filter) = search_key.filter {
            if filter.script_len_range.is_some() {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "doesn't support search_key.filter.script_len_range parameter",
                ))
                .into());
            }
            if filter.output_data_len_range.is_some() {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "doesn't support search_key.filter.output_data_len_range parameter",
                ))
                .into());
            }
            if filter.output_capacity_range.is_some() {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "doesn't support search_key.filter.output_capacity_range parameter",
                ))
                .into());
            }
            (filter.script, filter.block_range)
        } else {
            (None, None)
        };
        let (lock_script, type_script) = match search_key.script_type {
            indexer::ScriptType::Lock => (Some(script), the_other_script),
            indexer::ScriptType::Type => (the_other_script, Some(script)),
        };
        let block_range = block_range.map(|range| Range::new(range[0].into(), range[1].into()));

        let db_response = self
            .storage
            .get_indexer_transactions(lock_script, type_script, block_range, pagination)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()))?;

        Ok(indexer::PaginationResponse {
            objects: db_response.response,
            last_cursor: db_response.next_cursor.map(Into::into),
        })
    }

    pub(crate) async fn inner_get_live_cells_by_lock_hash(
        &self,
        lock_hash: H256,
        page: u64,
        per_page: u64,
        reverse_order: Option<bool>,
    ) -> InnerResult<Vec<indexer::LiveCell>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            if per_page > 50 {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                ))
                .into());
            }
            let skip = page * per_page;
            let limit = per_page as u16;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let cells = self
            .storage
            .get_live_cells(None, vec![lock_hash], vec![], None, pagination)
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

    pub(crate) async fn inner_get_capacity_by_lock_hash(
        &self,
        lock_hash: H256,
    ) -> InnerResult<indexer::LockHashCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, true);
        let db_response = self
            .storage
            .get_cells(None, vec![lock_hash], vec![], None, pagination)
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
            .get_tip()
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
    pub(crate) async fn inner_get_transactions_by_lock_hash(
        &self,
        lock_hash: H256,
        page: u64,
        per_page: u64,
        reverse_order: Option<bool>,
    ) -> InnerResult<Vec<indexer::CellTransaction>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            if per_page > 50 {
                return Err(CoreError::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                ))
                .into());
            }
            let skip = page * per_page;
            let limit = per_page as u16;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let db_response = self
            .storage
            .get_cells(None, vec![lock_hash], vec![], None, pagination)
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
                    .get_cells(Some(out_point), vec![], vec![], None, Default::default())
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

    pub(crate) async fn inner_get_transaction_with_status(
        &self,
        tx_hash: H256,
    ) -> InnerResult<TransactionWrapper> {
        let tx_wrapper = self
            .storage
            .get_transactions_by_hashes(vec![tx_hash.clone()], None, Default::default())
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

    pub(crate) async fn inner_get_transaction_info(
        &self,
        tx_hash: H256,
    ) -> InnerResult<GetTransactionInfoResponse> {
        let tx = self.inner_get_transaction_with_status(tx_hash).await?;
        let transaction = self.query_transaction_info(&tx).await?;

        Ok(GetTransactionInfoResponse {
            transaction: Some(transaction),
            status: TransactionStatus::Committed,
        })
    }

    async fn query_transaction_info(
        &self,
        tx_wrapper: &TransactionWrapper,
    ) -> InnerResult<TransactionInfo> {
        let mut records: Vec<Record> = vec![];

        let tip_block_number = **CURRENT_BLOCK_NUMBER.load();
        let tx_hash = tx_wrapper
            .transaction_with_status
            .transaction
            .clone()
            .expect("impossible: get transaction fail")
            .hash;

        for input_cell in &tx_wrapper.input_cells {
            let mut input_records = self
                .to_record(input_cell, IOType::Input, Some(tip_block_number))
                .await?;
            records.append(&mut input_records);
        }

        for output_cell in &tx_wrapper.output_cells {
            let mut output_records = self
                .to_record(output_cell, IOType::Output, Some(tip_block_number))
                .await?;
            records.append(&mut output_records);
        }

        let mut map: HashMap<H256, BigInt> = HashMap::new();
        for record in &records {
            let entry = map
                .entry(record.asset_info.udt_hash.clone())
                .or_insert_with(BigInt::zero);
            *entry += {
                let amount: u128 = record.amount.into();
                let amount: BigInt = amount.into();
                match record.io_type {
                    IOType::Input => amount.neg(),
                    IOType::Output => amount,
                }
            }
        }

        let fee = map
            .get(&H256::default())
            .map(|amount| -amount)
            .unwrap_or_else(BigInt::zero)
            .to_i64()
            .expect("impossible: get fee fail");

        // tips: according to the calculation rule, coinbase and dao claim transaction will get negative fee which is unreasonable.
        let fee = if fee < 0 { 0 } else { fee as u64 };

        let burn = map
            .iter()
            .filter(|(udt_hash, _)| **udt_hash != H256::default())
            .map(|(udt_hash, amount)| {
                let amount: u128 = if amount.sign() == Sign::Minus {
                    (-amount).to_u128().expect("get udt amount")
                } else {
                    0
                };
                BurnInfo {
                    udt_hash: udt_hash.to_owned(),
                    amount: amount.into(),
                }
            })
            .collect();

        Ok(TransactionInfo {
            tx_hash,
            records,
            fee: fee.into(),
            burn,
            timestamp: tx_wrapper.timestamp.into(),
        })
    }

    async fn get_live_cells_by_search_key(
        &self,
        search_key: indexer::SearchKey,
        pagination: PaginationRequest,
    ) -> InnerResult<common::PaginationResponse<DetailedCell>> {
        let script = search_key.script;
        let (
            the_other_script,
            script_len_range,
            output_data_len_range,
            output_capacity_range,
            block_range,
        ) = if let Some(filter) = search_key.filter {
            (
                filter.script,
                filter.script_len_range,
                filter.output_data_len_range,
                filter.output_capacity_range,
                filter.block_range,
            )
        } else {
            (None, None, None, None, None)
        };
        let (lock_script, type_script, lock_len_range, type_len_range) = match search_key
            .script_type
        {
            indexer::ScriptType::Lock => (Some(script), the_other_script, None, script_len_range),
            indexer::ScriptType::Type => (the_other_script, Some(script), script_len_range, None),
        };

        // Mercury uses [inclusive, inclusive] range
        let block_range = block_range.map(|range| {
            let to: u64 = range[1].into();
            Range::new(range[0].into(), to.saturating_sub(1))
        });
        let lock_len_range = lock_len_range.map(|range| {
            let to: u64 = range[1].into();
            Range::new(range[0].into(), to.saturating_sub(1))
        });
        let type_len_range = type_len_range.map(|range| {
            let to: u64 = range[1].into();
            Range::new(range[0].into(), to.saturating_sub(1))
        });
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
            .get_live_cells_ex(
                lock_script,
                type_script,
                lock_len_range,
                type_len_range,
                block_range,
                capacity_range,
                data_len_range,
                pagination,
            )
            .await;
        let db_response = db_response.map_err(|error| CoreError::DBError(error.to_string()))?;
        Ok(db_response)
    }

    pub(crate) async fn inner_get_sync_state(&self) -> InnerResult<SyncState> {
        let state = (&*self.sync_state.read()).to_owned();
        match state {
            SyncState::ReadOnly => Ok(state.to_owned()),
            SyncState::ParallelFirstStage(sync_process) => {
                let current_count = self
                    .storage
                    .block_count()
                    .await
                    .map_err(|error| CoreError::DBError(error.to_string()))?;
                let target = sync_process.target.parse::<u64>().expect("get sync target");
                let state = SyncState::ParallelFirstStage(SyncProgress::new(
                    current_count.saturating_sub(1),
                    target,
                    utils::calculate_the_percentage(current_count.saturating_sub(1), target),
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
