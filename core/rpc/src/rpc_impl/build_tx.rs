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
        let live_acps = self
            .get_live_cells_by_item(
                item.clone(),
                payload.asset_info.clone(),
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
        fee_rate: u64,
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

        // let inputs = if from.is_empty() {
        //     self.get_pool_live_cells_by_item(item, ckb_needs as i64, vec![], None)
        //         .await?
        // } else {
        //     self.try_pool_ckb_by_items(from, ckb_needs as i64).await?
        // };

        // let script_set = HashSet::new();
        // script_set.insert(
        //     self.builtin_scripts
        //         .get(SECP256K1)
        //         .cloned()
        //         .unwrap()
        //         .cell_dep,
        // );

        // script_set.insert(self.builtin_scripts.get(SUDT).cloned().unwrap().cell_dep);

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
        inputs: &[DetailedCell],
        fee_rate: u64,
        script_set: &mut HashSet<packed::CellDep>,
    ) -> InnerResult<TransactionCompletionResponse> {
        //let mut script_set = HashSet::new();
        let mut sig_entries = HashMap::new();

        for (idx, input) in inputs.iter().enumerate() {
            let code_hash: H256 = input.cell_output.lock().code_hash().unpack();
            if code_hash == **SECP256K1_CODE_HASH.load() {
                let addr = self
                    .script_to_address(&input.cell_output.lock())
                    .to_string();
                add_sig_entry(
                    addr,
                    input.cell_output.calc_lock_hash().to_string(),
                    &mut sig_entries,
                    idx,
                );
            } else if code_hash == **ACP_CODE_HASH.load() {
                let pub_key_hash =
                    H160::from_slice(&input.cell_output.lock().args().raw_data()[0..20]).unwrap();
                let addr = Address::new(
                    self.network_type,
                    AddressPayload::from_pubkey_hash(self.network_type, pub_key_hash),
                );
                add_sig_entry(
                    addr.to_string(),
                    input.cell_output.calc_lock_hash().to_string(),
                    &mut sig_entries,
                    idx,
                );
            } else if code_hash == **CHEQUE_CODE_HASH.load() {
            } else if code_hash == **DAO_CODE_HASH.load() {
                todo!()
            }
        }

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

fn add_sig_entry(
    address: String,
    lock_hash: String,
    sigs_entry: &mut HashMap<String, SignatureEntry>,
    index: usize,
) {
    if let Some(entry) = sigs_entry.get_mut(&lock_hash) {
        entry.add_group();
    } else {
        sigs_entry.insert(
            lock_hash.clone(),
            SignatureEntry {
                type_: WitnessType::WitnessLock,
                group_len: 1,
                pub_key: address,
                sig_type: SignatureType::Secp256k1,
                index,
            },
        );
    }
}
