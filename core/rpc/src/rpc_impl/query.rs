use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, utils, CURRENT_BLOCK_NUMBER,
};
use crate::types::{self,
    indexer,
    AddressOrLockHash, AssetInfo, AssetType, Balance, BlockInfo, BurnInfo, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, IOType, Item, QueryTransactionsPayload, Record, StructureType,
    TransactionInfo, TransactionStatus, TxView,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, parse_address, to_fixed_array};
use common::{
    hash::blake2b_160, Address, AddressPayload, MercuryError, Order, PaginationRequest,
    PaginationResponse, Range, Result, SECP256K1,
};
use core_storage::{DBInfo, Storage};

use bincode::deserialize;
use ckb_jsonrpc_types::{self, Capacity, CellDep, CellOutput, JsonBytes, OutPoint, Script, TransactionWithStatus, Uint32, Uint64};
use ckb_types::core::{self, BlockNumber, RationalU256, TransactionView};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use lazysort::SortedBy;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};
use std::{str::FromStr, thread::ThreadId};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info()
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))
    }

    pub(crate) async fn inner_get_balance(
        &self,
        payload: GetBalancePayload,
    ) -> InnerResult<GetBalanceResponse> {
        let item: Item = payload.item.try_into()?;
        let tip_epoch_number = if let Some(tip_block_number) = payload.tip_block_number {
            Some(self.get_epoch_by_number(tip_block_number).await?)
        } else {
            None
        };

        let live_cells = self
            .get_live_cells_by_item(
                item.clone(),
                payload.asset_infos.clone(),
                payload.tip_block_number,
                tip_epoch_number.clone(),
                None,
                None,
            )
            .await?;

        let mut balances_map: HashMap<(AddressOrLockHash, AssetInfo), Balance> = HashMap::new();

        let secp_lock_hash = self.get_secp_lock_hash_by_item(item)?;

        for cell in live_cells {
            let records = self
                .to_record(
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
                                .args(Bytes::from((&args[0..20]).to_vec()).pack())
                                .build()
                                .calc_script_hash()
                                .unpack();
                            secp_lock_hash == H160::from_slice(&lock_hash.0[0..20]).unwrap()
                        }
                        AddressOrLockHash::LockHash(lock_hash) => {
                            secp_lock_hash == H160::from_str(lock_hash).unwrap()
                        }
                    }
                })
                .collect();

            self.accumulate_balance_from_records(
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

    pub(crate) async fn inner_get_block_info(
        &self,
        payload: GetBlockInfoPayload,
    ) -> InnerResult<BlockInfo> {
        let block_info = self
            .storage
            .get_simple_block(payload.block_hash, payload.block_number)
            .await;
        let block_info = match block_info {
            Ok(block_info) => block_info,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
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
            block_number: block_info.block_number,
            block_hash: block_info.block_hash,
            parent_hash: block_info.parent_hash,
            timestamp: block_info.timestamp,
            transactions,
        })
    }

    pub(crate) async fn inner_query_transaction(
        &self,
        payload: QueryTransactionsPayload,
    ) -> InnerResult<PaginationResponse<TxView>> {
        let pagination_ret = self
            .get_transactions_by_item(
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
                    .map(|tx_view| {
                        let hash = H256::from_slice(tx_view.hash().as_slice()).unwrap();
                        TxView::TransactionView(TransactionWithStatus::with_committed(
                            tx_view, hash,
                        ))
                    })
                    .collect(),
                next_cursor: pagination_ret.next_cursor,
                count: pagination_ret.count,
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
                    next_cursor: pagination_ret.next_cursor,
                    count: pagination_ret.count,
                })
            }
        }
    }

    pub(crate) async fn inner_get_tip(&self) -> InnerResult<Option<indexer::Tip>> {
        let block = self
            .storage
            .get_tip()
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

    pub(crate) async fn inner_get_cells(
        &self,
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
            .get_cells_by_search_key(search_key, pagination, true)
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

    pub(crate) async fn inner_get_spent_transaction(
        &self,
        payload: GetSpentTransactionPayload,
    ) -> InnerResult<TxView> {
        match &payload.structure_type {
            StructureType::Native => self.get_spent_transaction_view(payload.outpoint).await,
            StructureType::DoubleEntry => {
                let tx_hash = self
                    .storage
                    .get_spent_transaction_hash(payload.outpoint.into())
                    .await
                    .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
                let tx_hash = match tx_hash {
                    Some(tx_hash) => tx_hash,
                    None => return Err(RpcErrorMessage::CannotFindSpentTransaction),
                };
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
        payload: indexer::SearchKey,
    ) -> InnerResult<indexer::CellsCapacity> {
        let pagination = PaginationRequest::new(None, Order::Asc, None, None, false);
        let db_response = self
            .get_cells_by_search_key(payload, pagination, true)
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
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        if block.is_none() {
            return Err(RpcErrorMessage::DBError(String::from(
                "fail to get tip block",
            )));
        }
        let (block_number, block_hash) = block.unwrap();

        Ok(indexer::CellsCapacity {
            capacity: capacity.into(),
            block_hash,
            block_number: block_number.into(),
        })
    }

    pub(crate) async fn inner_get_transaction(
        &self,
        search_key: indexer::SearchKey,
        order: indexer::Order,
        limit: Uint64,
        after_cursor: Option<Bytes>,
    ) -> InnerResult<indexer::PaginationResponse<indexer::Transaction>> {
        let pagination = {
            let order: common::Order = order.into();
            PaginationRequest::new(after_cursor, order, Some(limit.into()), None, false)
        };
        let db_response = self
            .get_cells_by_search_key(search_key, pagination, false)
            .await?;

        let mut objects: Vec<indexer::Transaction> = vec![];
        for cell in db_response.response.iter() {
            let out_cell = cell.clone();
            let object = indexer::Transaction {
                tx_hash: out_cell.out_point.tx_hash().unpack(),
                block_number: out_cell.block_number.into(),
                tx_index: out_cell.tx_index.into(),
                io_index: out_cell.out_point.index().unpack(),
                io_type: indexer::IOType::Output,
            };
            objects.push(object);
            let out_point = cell.out_point.clone();
            let consume_info = self
                .storage
                .get_cells(Some(out_point), vec![], vec![], None, Default::default())
                .await
                .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?
                .response;
            if let Some(detailed_cell) = consume_info.get(0).cloned() {
                let object = indexer::Transaction {
                    tx_hash: cell.out_point.tx_hash().unpack(),
                    block_number: cell.block_number.into(),
                    tx_index: cell.tx_index.into(),
                    io_index: detailed_cell
                        .consumed_input_index
                        .ok_or(RpcErrorMessage::MissingConsumedInfo)?.into(),
                    io_type: indexer::IOType::Input,
                };
                objects.push(object);
            };
        }
        Ok(indexer::PaginationResponse {
            objects,
            last_cursor: db_response.next_cursor,
        })
    }

    pub(crate) async fn inner_get_live_cells_by_lock_hash(
        &self,
        lock_hash: H256,
        page: Uint64,
        per_page: Uint64,
        reverse_order: Option<bool>,
    ) -> InnerResult<Vec<types::indexer_legacy::LiveCell>> {
        let pagination = {
            let order = match reverse_order {
                Some(true) => Order::Desc,
                _ => Order::Asc,
            };
            let page: u64 = page.into();
            let per_page: u64 = per_page.into();
            let skip = page * per_page;
            let limit = per_page;
            PaginationRequest::new(None, order, Some(limit), Some(skip), false)
        };
        let cells = self.storage.get_live_cells(
            None,
            vec![lock_hash],
            vec![],
            None,
            pagination,
        ).await
        .map_err(|error| RpcErrorMessage::DBError(error.to_string()))?;
        let res: Vec<types::indexer_legacy::LiveCell> = cells.response.into_iter()
        .map(|cell| {
            let index: u32 = cell.out_point.index().unpack();
            let tranaction_point = types::indexer_legacy::TransactionPoint {
                block_number: cell.block_number.into(),
                tx_hash: cell.out_point.tx_hash().unpack(),
                index: index.into(),
            };
            types::indexer_legacy::LiveCell {
                created_by: tranaction_point,
                cell_output: cell.cell_output.into(),
                output_data_len: (cell.cell_data.len() as u64).into(),
                cellbase: cell.tx_index == 0,
            }
        }
        ).collect();
        Ok(res)
    }

    pub(crate) async fn get_spent_transaction_view(
        &self,
        outpoint: OutPoint,
    ) -> InnerResult<TxView> {
        let tx_view = self
            .storage
            .get_transactions(
                vec![outpoint.tx_hash],
                vec![],
                vec![],
                None,
                PaginationRequest::default().set_limit(Some(1)),
            )
            .await;
        let tx_view = match tx_view {
            Ok(tx_view) => tx_view,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        let tx_view = match tx_view.response.get(0).cloned() {
            Some(tx_view) => tx_view,
            None => return Err(RpcErrorMessage::CannotFindSpentTransaction),
        };
        let tx_info = self
            .storage
            .get_simple_transaction_by_hash(tx_view.hash().unpack())
            .await;
        let block_hash = match tx_info {
            Ok(tx_info) => tx_info.block_hash,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        Ok(TxView::TransactionView(
            TransactionWithStatus::with_committed(tx_view, block_hash),
        ))
    }

    pub(crate) async fn inner_get_transaction_info(
        &self,
        tx_hash: H256,
    ) -> InnerResult<GetTransactionInfoResponse> {
        let tx_view = self
            .storage
            .get_transactions(
                vec![tx_hash.clone()],
                vec![],
                vec![],
                None,
                Default::default(),
            )
            .await;
        let tx_view = match tx_view {
            Ok(tx_view) => tx_view,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        let tx_view = match tx_view.response.get(0).cloned() {
            Some(tx_view) => tx_view,
            None => return Err(RpcErrorMessage::CannotFindTransactionByHash),
        };
        let transaction = self.query_transaction_info(&tx_view).await?;
        Ok(GetTransactionInfoResponse {
            transaction: Some(transaction),
            status: TransactionStatus::Committed,
            reject_reason: None,
        })
    }

    async fn query_transaction_info(
        &self,
        tx_view: &TransactionView,
    ) -> InnerResult<TransactionInfo> {
        let mut records: Vec<Record> = vec![];

        let tip = self.storage.get_tip().await;
        let tip = match tip {
            Ok(tip) => tip,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        let tip_block_number = match tip {
            Some((tip_block_number, _)) => tip_block_number,
            None => return Err(RpcErrorMessage::DBError(String::new())),
        };
        let tip_epoch_number = self.get_epoch_by_number(tip_block_number).await?;

        let input_pts: Vec<packed::OutPoint> = tx_view
            .inputs()
            .into_iter()
            .map(|cell| cell.previous_output())
            .collect();
        self.out_points_to_records(
            input_pts,
            IOType::Input,
            tip_block_number,
            tip_epoch_number.clone(),
            &mut records,
        )
        .await?;

        self.out_points_to_records(
            tx_view.output_pts(),
            IOType::Output,
            tip_block_number,
            tip_epoch_number,
            &mut records,
        )
        .await?;

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
        let fee = if tx_view.is_cellbase() {
            0
        } else {
            map.get(&H256::default())
                .map(|amount| -amount)
                .unwrap_or_else(BigInt::zero)
                .to_u64()
                .expect("impossible: get fee fail")
        };

        Ok(TransactionInfo {
            tx_hash: H256(to_fixed_array::<32>(&tx_view.hash().as_bytes())),
            records,
            fee,
            burn: map
                .iter()
                .map(|(udt_hash, amount)| BurnInfo {
                    udt_hash: udt_hash.to_owned(),
                    amount: (-amount).to_string(),
                })
                .collect(),
        })
    }

    async fn out_points_to_records(
        &self,
        pts: Vec<packed::OutPoint>,
        io_type: IOType,
        tip_block_number: u64,
        tip_epoch_number: RationalU256,
        output_records: &mut Vec<Record>,
    ) -> InnerResult<()> {
        for pt in pts {
            if pt.tx_hash().is_zero() {
                continue;
            }

            let detailed_cell = self
                .storage
                .get_cells(
                    Some(pt),
                    vec![],
                    vec![],
                    None,
                    PaginationRequest::default().set_limit(Some(1)),
                )
                .await;

            let detailed_cell = match detailed_cell {
                Ok(detailed_cell) => detailed_cell,
                Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
            };
            let detailed_cell = match detailed_cell.response.get(0).cloned() {
                Some(detailed_cell) => detailed_cell,
                None => return Err(RpcErrorMessage::CannotFindDetailedCellByOutPoint),
            };

            let mut records = self
                .to_record(
                    &detailed_cell,
                    io_type.clone(),
                    Some(tip_block_number),
                    Some(tip_epoch_number.clone()),
                )
                .await?;
            output_records.append(&mut records);
        }

        Ok(())
    }

    async fn get_cells_by_search_key(
        &self,
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
                .get_live_cells(None, lock_hashes, type_hashes, block_range, pagination)
                .await
        } else {
            self.storage
                .get_cells(None, lock_hashes, type_hashes, block_range, pagination)
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
