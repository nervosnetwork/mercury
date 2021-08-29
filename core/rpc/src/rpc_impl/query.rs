use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, utils, CURRENT_BLOCK_NUMBER,
};
use crate::types::{
    Balance, GetBalanceResponse, GetSpentTransactionPayload, IOType, Record, TransactionInfo,
    TxView,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError, Order};
use core_storage::{DBAdapter, DBInfo, MercuryStore};

use bincode::deserialize;
use ckb_jsonrpc_types::{CellDep, CellOutput, OutPoint, Script, TransactionWithStatus};
use ckb_types::core::{self, BlockNumber, RationalU256, TransactionView};
use ckb_types::{packed, prelude::*, H160, H256};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

use lazysort::SortedBy;
use num_traits::Zero;

impl<C: CkbRpc + DBAdapter> MercuryRpcImpl<C> {
    pub(crate) fn inner_get_db_info(&self) -> InnerResult<DBInfo> {
        self.storage
            .get_db_info()
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))
    }

    pub(crate) async fn get_spent_transaction_view(
        &self,
        outpoint: OutPoint,
    ) -> InnerResult<TxView> {
        let tx_view = self
            .storage
            .get_spent_transaction_view(outpoint.into())
            .await;
        let tx_view = match tx_view {
            Ok(tx_view) => tx_view,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        let tx_view = match tx_view {
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
        let mut capacity_sum_inputs: u64 = 0;
        let mut capacity_sum_outputs: u64 = 0;

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
            &mut capacity_sum_inputs,
        )
        .await?;

        self.out_points_to_records(
            tx_view.output_pts(),
            IOType::Output,
            tip_block_number,
            tip_epoch_number,
            &mut records,
            &mut &mut capacity_sum_outputs,
        )
        .await?;

        let tx_hash = H256(to_fixed_array::<32>(&tx_view.hash().as_bytes()));

        Ok(TransactionInfo {
            tx_hash,
            records,
            fee: capacity_sum_inputs - capacity_sum_outputs,
            burn: vec![],
        })
    }

    async fn out_points_to_records(
        &self,
        pts: Vec<packed::OutPoint>,
        io_type: IOType,
        tip_block_number: u64,
        tip_epoch_number: RationalU256,
        output_records: &mut Vec<Record>,
        capacity_sum: &mut u64,
    ) -> InnerResult<()> {
        for pt in pts {
            let detailed_cell = self.storage.get_detailed_cell(pt).await;
            let detailed_cell = match detailed_cell {
                Ok(detailed_cell) => detailed_cell,
                Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
            };
            let detailed_cell = match detailed_cell {
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
            let capacity: u64 = detailed_cell.cell_output.capacity().unpack();
            *capacity_sum += capacity;
        }
        Ok(())
    }
}
