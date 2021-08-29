use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER,
    DAO_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH,
};
use crate::types::{
    decode_record_id, encode_record_id, AdjustAccountPayload, AssetInfo, AssetType, DaoInfo,
    DaoState, ExtraFilter, IOType, Identity, IdentityFlag, Item, JsonItem, Record, RequiredUDT,
    SignatureEntry, SignatureType, Source, Status, TransactionCompletionResponse, WitnessType,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_dao_block_number, decode_udt_amount, parse_address};
use common::{
    Address, AddressPayload, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1, SUDT,
};
use core_storage::DBAdapter;

use ckb_types::core::{
    BlockNumber, EpochNumberWithFraction, RationalU256, ScriptHashType, TransactionView,
};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

const BYTE_SHANNONS: u64 = 100_000_000;
const STANDARD_SUDT_CAPACITY: u64 = 142 * BYTE_SHANNONS;

const fn ckb(num: u64) -> u64 {
    num * BYTE_SHANNONS
}

impl<C: CkbRpc + DBAdapter> MercuryRpcImpl<C> {
    pub(crate) async fn inner_build_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> InnerResult<Option<TransactionCompletionResponse>> {
        if payload.asset_info.asset_type == AssetType::Ckb {
            return Err(RpcErrorMessage::AdjustAccountOnCkb);
        }

        let account_number = payload.account_number.clone().unwrap_or(1) as usize;
        let extra_ckb = payload.extra_ckb.clone().unwrap_or(1);
        let fee_rate = payload.fee_rate.clone().unwrap_or(1000);

        let item: Item = payload.item.clone().try_into()?;
        let mut asset_set = HashSet::new();
        asset_set.insert(payload.asset_info.clone());
        let live_acps = self
            .get_live_cells_by_item(
                item.clone(),
                asset_set,
                None,
                None,
                Some((**ACP_CODE_HASH.load()).clone()),
                None,
            )
            .await?;
        let live_acps_len = live_acps.len();

        if live_acps_len == account_number {
            return Ok(None);
        }

        let sudt_type_script = self.build_sudt_type_script(payload.asset_info.udt_hash.0.to_vec());
        let from = parse_from(payload.from.clone())?;

        if live_acps_len < account_number {
            self.build_create_acp_transaction(
                from,
                account_number - live_acps_len,
                sudt_type_script,
                item,
                extra_ckb,
                fee_rate,
            )
            .await?;
        } else {
        }

        todo!()
    }

    async fn build_create_acp_transaction(
        &self,
        from: Vec<Item>,
        acp_need_count: usize,
        sudt_type_script: packed::Script,
        item: Item,
        extra_ckb: u64,
        _fee_rate: u64,
    ) -> InnerResult<()> {
        let mut ckb_needs = ckb(1);
        let (mut outputs, mut outputs_data) = (vec![], vec![]);
        for _i in 0..acp_need_count {
            let capacity = STANDARD_SUDT_CAPACITY + ckb(extra_ckb);
            let output_cell = self.build_acp_output(
                Some(sudt_type_script.clone()),
                self.get_secp_lock_hash_by_item(item.clone())?.0.to_vec(),
                capacity,
            );
            outputs.push(output_cell);
            outputs_data.push(Bytes::new());
            ckb_needs += capacity;
        }

        let mut inputs = Vec::new();
        let mut script_set = HashSet::new();
        let mut sig_entries = HashMap::new();

        if from.is_empty() {
            self.get_pool_live_cells_by_items(
                vec![item],
                ckb_needs as i64,
                vec![],
                None,
                &mut inputs,
                &mut script_set,
                &mut sig_entries,
            )
            .await?;
        } else {
            self.get_pool_live_cells_by_items(
                from,
                ckb_needs as i64,
                vec![],
                None,
                &mut inputs,
                &mut script_set,
                &mut sig_entries,
            )
            .await?;
        }

        script_set.insert(SECP256K1.to_string());
        script_set.insert(SUDT.to_string());

        Ok(())
    }

    fn build_acp_output(
        &self,
        type_script: Option<packed::Script>,
        lock_args: Vec<u8>,
        capacity: u64,
    ) -> packed::CellOutput {
        let lock_script = self
            .builtin_scripts
            .get(ACP)
            .cloned()
            .unwrap()
            .script
            .as_builder()
            .args(lock_args.pack())
            .build();
        packed::CellOutputBuilder::default()
            .type_(type_script.pack())
            .lock(lock_script)
            .capacity(capacity.pack())
            .build()
    }

    fn build_sudt_type_script(&self, type_args: Vec<u8>) -> packed::Script {
        self.builtin_scripts
            .get(SUDT)
            .cloned()
            .unwrap()
            .script
            .as_builder()
            .args(type_args.pack())
            .build()
    }

    fn build_tx_complete_resp(
        &self,
        fee_rate: u64,
        inputs: &[DetailedCell],
        script_set: &mut HashSet<String>,
        sig_entries: &mut HashMap<String, SignatureEntry>,
    ) -> InnerResult<TransactionCompletionResponse> {
        todo!()
    }
}

fn parse_from(from_set: HashSet<JsonItem>) -> InnerResult<Vec<Item>> {
    let mut ret: Vec<Item> = Vec::new();
    for ji in from_set.into_iter() {
        ret.push(ji.try_into()?);
    }

    Ok(ret)
}
