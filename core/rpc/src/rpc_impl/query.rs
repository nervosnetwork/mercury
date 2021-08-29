use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, CURRENT_BLOCK_NUMBER,
};
use crate::types::{
    Balance, GetBalanceResponse, GetSpentTransactionPayload, TransactionInfo, TxView,
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
    ) -> TransactionInfo {
        // let records: Vec<Record> = vec![];

        // let cell_inputs = tx_view.inputs();
        // for cell_input in cell_inputs {
        //     let outpoint = cell_input.previous_output();
        //     let a = self.storage.get_detailed_cell(outpoint).await;
        // }
        todo!()
    }
}
