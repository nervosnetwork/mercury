use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, utils, CURRENT_BLOCK_NUMBER,
};
use crate::types::{
    AssetType, Balance, BurnInfo, GetBalanceResponse, GetSpentTransactionPayload, IOType,
    QueryTransactionsPayload, Record, StructureType, TransactionInfo, TxView,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{
    hash::blake2b_160, Address, AddressPayload, MercuryError, Order, PaginationRequest,
    PaginationResponse, Result,
};
use core_storage::{DBInfo, Storage};

use bincode::deserialize;
use ckb_jsonrpc_types::{CellDep, CellOutput, OutPoint, Script, TransactionWithStatus};
use ckb_types::core::{self, BlockNumber, RationalU256, TransactionView};
use ckb_types::{packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

use lazysort::SortedBy;
use num_traits::{ToPrimitive, Zero};

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info()
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))
    }

    pub(crate) async fn inner_query_transaction(
        &self,
        payload: QueryTransactionsPayload,
    ) -> InnerResult<PaginationResponse<TxView>> {
        let pagination_ret = self
            .get_transactions_by_item(
                payload.item.try_into()?,
                payload.asset_types,
                payload.extra_filter,
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
                        let hash = H256::from_slice(&tx_view.hash().as_slice()).unwrap();
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

    pub(crate) async fn query_transaction_info(
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
        let fee = map
            .get(&H256::default())
            .map(|amount| -amount)
            .unwrap_or_else(BigInt::zero)
            .to_u64()
            .expect("impossible: get fee fail");

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
            let detailed_cell = self
                .storage
                .get_live_cells(
                    Some(pt),
                    vec![],
                    vec![],
                    None,
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
                    tip_block_number,
                    tip_epoch_number.clone(),
                )
                .await?;
            output_records.append(&mut records);
        }
        Ok(())
    }
}
