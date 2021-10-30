use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{address_to_script, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER};
use crate::types::{
    indexer, indexer_legacy, AddressOrLockHash, AssetInfo, Balance, BlockInfo, BurnInfo,
    GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, IOType, Item, QueryTransactionsPayload, Record, StructureType,
    TransactionInfo, TransactionStatus, TxView,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::parse_address;
use common::{Context, Order, PaginationRequest, PaginationResponse, Range, SECP256K1};
use common_logger::tracing_async;
use core_storage::{DBInfo, Storage};

use ckb_jsonrpc_types::{self, Capacity, Script, Uint64};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

use protocol::TransactionWrapper;
use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, str::FromStr};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self, ctx: Context) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info(ctx)
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))
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
                true,
            )
            .await?;

        let mut balances_map: HashMap<(AddressOrLockHash, AssetInfo), Balance> = HashMap::new();

        let secp_lock_hash = self.get_secp_lock_hash_by_item(item)?;

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

            // filter record, remain the one that owned by item.
            let records: Vec<Record> = records
                .into_iter()
                .filter(|record| {
                    match &record.address_or_lock_hash {
                        AddressOrLockHash::Address(address) => {
                            // unwrap here is ok, because if this address is invalid, it will throw error for more earlier.
                            let address = parse_address(address).unwrap();
                            let args: Bytes = address_to_script(address.payload()).args().unpack();
                            let lock_hash: H256 = self
                                .get_script_builder(SECP256K1)
                                .unwrap()
                                .args(Bytes::from((&args[0..20]).to_vec()).pack())
                                .build()
                                .calc_script_hash()
                                .unpack();
                            secp_lock_hash == H160::from_slice(&lock_hash.0[0..20]).unwrap()
                        }
                        AddressOrLockHash::LockHash(lock_hash) => {
                            secp_lock_hash
                                == H160::from_str(lock_hash)
                                    .map_err(|_| {
                                        RpcErrorMessage::InvalidScriptHash(lock_hash.clone())
                                    })
                                    .unwrap()
                        }
                    }
                })
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
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
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
                    .map(|tx_wrapper| TxView::TransactionView(tx_wrapper.transaction_with_status))
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
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
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
            .get_cells_by_search_key(ctx.clone(), search_key, pagination, true)
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
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        let tx_hash = match tx_hash {
            Some(tx_hash) => tx_hash,
            None => return Err(RpcErrorMessage::CannotFindSpentTransaction),
        };

        match &payload.structure_type {
            StructureType::Native => {
                let tx = self
                    .inner_get_transaction_with_status(ctx.clone(), tx_hash)
                    .await?;
                Ok(TxView::TransactionView(tx.transaction_with_status))
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
            .get_cells_by_search_key(ctx.clone(), payload, pagination, true)
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
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        let (block_number, block_hash) =
            block.ok_or_else(|| RpcErrorMessage::DBError(String::from("fail to get tip block")))?;

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
                lock_script,
                type_script,
                block_range,
                pagination,
            )
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;

        let mut objects = Vec::new();
        for cell in db_response.response.iter() {
            let object = indexer::Transaction {
                tx_hash: H256::from_slice(&cell.tx_hash.rb_bytes[0..32]).unwrap(),
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
    ) -> InnerResult<Vec<indexer_legacy::LiveCell>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            let page: u64 = page.into();
            let per_page: u64 = per_page.into();
            if per_page > 50 {
                return Err(RpcErrorMessage::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                )));
            }
            let skip = page * per_page;
            let limit = per_page;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let cells = self
            .storage
            .get_live_cells(ctx.clone(), None, vec![lock_hash], vec![], None, pagination)
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        let res: Vec<indexer_legacy::LiveCell> = cells
            .response
            .into_iter()
            .map(|cell| {
                let index: u32 = cell.out_point.index().unpack();
                let tranaction_point = indexer_legacy::TransactionPoint {
                    block_number: cell.block_number.into(),
                    tx_hash: cell.out_point.tx_hash().unpack(),
                    index: index.into(),
                };
                indexer_legacy::LiveCell {
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
    ) -> InnerResult<indexer_legacy::LockHashCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, true);
        let db_response = self
            .storage
            .get_cells(ctx.clone(), None, vec![lock_hash], vec![], None, pagination)
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;

        let cells_count = db_response
            .count
            .ok_or_else(|| RpcErrorMessage::DBError(String::from("fail to get cells count")))?;
        let capacity: u64 = db_response
            .response
            .into_iter()
            .map(|cell| -> u64 { cell.cell_output.capacity().unpack() })
            .sum();

        let block = self
            .storage
            .get_tip(ctx.clone())
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        let (block_number, _) =
            block.ok_or_else(|| RpcErrorMessage::DBError(String::from("fail to get tip block")))?;

        Ok(indexer_legacy::LockHashCapacity {
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
    ) -> InnerResult<Vec<indexer_legacy::CellTransaction>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            let page: u64 = page.into();
            let per_page: u64 = per_page.into();
            if per_page > 50 {
                return Err(RpcErrorMessage::InvalidRpcParams(String::from(
                    "per_page exceeds maximum page size 50",
                )));
            }
            let skip = page * per_page;
            let limit = per_page;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let db_response = self
            .storage
            .get_cells(ctx.clone(), None, vec![lock_hash], vec![], None, pagination)
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;

        let mut cell_txs: Vec<indexer_legacy::CellTransaction> = vec![];
        for cell in db_response.response.iter() {
            let out_cell = cell.clone();
            let created_by = indexer_legacy::TransactionPoint {
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
                    .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?
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
                        Some(indexer_legacy::TransactionPoint {
                            block_number: block_number.unwrap().into(),
                            tx_hash: tx_hash.unwrap(),
                            index: index.unwrap().into(),
                        })
                    }
                } else {
                    None
                }
            };

            cell_txs.push(indexer_legacy::CellTransaction {
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
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        let tx_wrapper = match tx_wrapper.response.get(0).cloned() {
            Some(tx_wrapper) => tx_wrapper,
            None => return Err(RpcErrorMessage::CannotFindTransactionByHash),
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
            .unwrap()
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
        })
    }

    #[tracing_async]
    async fn get_cells_by_search_key(
        &self,
        ctx: Context,
        search_key: indexer::SearchKey,
        pagination: PaginationRequest,
        only_live_cells: bool,
    ) -> InnerResult<PaginationResponse<common::DetailedCell>> {
        let script = search_key.script;
        let (the_other_script, output_data_len_range, output_capacity_range, block_range) =
            if let Some(filter) = search_key.filter {
                (
                    filter.script,
                    filter.output_capacity_range,
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

        let db_response = if only_live_cells {
            self.storage
                .get_live_cells(
                    ctx.clone(),
                    None,
                    lock_hashes,
                    type_hashes,
                    block_range,
                    pagination,
                )
                .await
        } else {
            self.storage
                .get_cells(
                    ctx.clone(),
                    None,
                    lock_hashes,
                    type_hashes,
                    block_range,
                    pagination,
                )
                .await
        };
        let mut db_response =
            db_response.map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;

        let data_len: [u64; 2] = if let Some(range) = output_data_len_range {
            [range[0].into(), range[1].into()]
        } else {
            [0, 0]
        };
        let capacity_len: [u64; 2] = if let Some(range) = output_capacity_range {
            [range[0].into(), range[1].into()]
        } else {
            [0, 0]
        };

        db_response.response = db_response
            .response
            .into_iter()
            .filter(|cell| {
                if data_len[1] != 0 {
                    let cell_data_len = cell.cell_data.len() as u64;
                    if cell_data_len < data_len[0] || cell_data_len >= data_len[1] {
                        return false;
                    }
                }
                if capacity_len[1] != 0 {
                    let capacity_data_len: u64 = cell.cell_output.capacity().unpack();
                    if capacity_data_len < capacity_len[0] || capacity_data_len >= capacity_len[1] {
                        return false;
                    }
                }
                true
            })
            .collect();

        Ok(db_response)
    }
}
