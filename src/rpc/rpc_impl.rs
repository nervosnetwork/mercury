use crate::extensions::{
    ckb_balance, rce_validator, special_cells, udt_balance, DetailedCell, DetailedCells,
    CKB_EXT_PREFIX, RCE_EXT_PREFIX, SP_CELL_EXT_PREFIX, UDT_EXT_PREFIX,
};
use crate::rpc::types::{
    details_split_off, CelllWithData, DetailedAmount, InnerAccount, InnerTransferItem,
    InputConsume, ScriptType, SignatureEntry, TransferCompletionResponse, TransferPayload,
    WitnessType, CHEQUE,
};
use crate::rpc::MercuryRpc;
use crate::utils::{
    decode_udt_amount, encode_udt_amount, parse_address, to_fixed_array, u128_sub, u64_sub,
    unwrap_only_one,
};
use crate::{error::MercuryError, stores::add_prefix, types::DeployedScriptConfig};

use anyhow::Result;
use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_indexer::store::{IteratorDirection, Store};
use ckb_sdk::{Address, AddressPayload};
use ckb_types::core::{BlockNumber, Capacity, ScriptHashType, TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use std::collections::{HashMap, HashSet};
use std::thread::{self, ThreadId};
use std::{convert::TryInto, iter::Iterator, str::FromStr};

const BYTE_SHANNONS: u64 = 100_000_000;
const MIN_CKB_CAPACITY: u64 = 61 * BYTE_SHANNONS;

lazy_static::lazy_static! {
    static ref ACP_USED_CACHE: DashMap<ThreadId, (Vec<packed::OutPoint>, u64)> = DashMap::new();
}

macro_rules! rpc_try {
    ($input: expr) => {
        $input.map_err(|e| Error::invalid_params(e.to_string()))?
    };
}

pub struct MercuryRpcImpl<S> {
    store: S,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> MercuryRpc for MercuryRpcImpl<S>
where
    S: Store + Send + Sync + 'static,
{
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.ckb_balance(&address));
        Ok(ret)
    }

    fn get_sudt_balance(&self, sudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(&address, sudt_hash));
        Ok(ret)
    }

    fn get_xudt_balance(&self, xudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(&address, xudt_hash));
        Ok(ret)
    }

    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool> {
        let key = rce_validator::Key::Address(&rce_hash.pack(), &addr.pack()).into_vec();

        self.store
            .get(&add_prefix(*RCE_EXT_PREFIX, key))
            .map_or_else(|_| Err(Error::internal_error()), |res| Ok(res.is_some()))
    }

    fn transfer_completion(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransferCompletionResponse> {
        self.inner_transfer_complete(
            payload.udt_hash.clone(),
            payload.from.to_inner(),
            payload.to_inner_items(),
            payload.change.clone(),
            payload.fee,
        )
        .map_err(|e| Error::invalid_params(e.to_string()))
    }
}

impl<S: Store> MercuryRpcImpl<S> {
    pub fn new(store: S, config: HashMap<String, DeployedScriptConfig>) -> Self {
        MercuryRpcImpl { store, config }
    }

    fn inner_transfer_complete(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransferCompletionResponse> {
        let mut amounts = DetailedAmount::new();
        let mut output_capacity = 0u64;
        let mut scripts_set = from.scripts.clone().into_iter().collect::<HashSet<_>>();
        let (mut outputs, mut cell_data) = (vec![], vec![]);
        let change = change.unwrap_or_else(|| from.idents[0].clone());

        for item in items.iter() {
            let addr = unwrap_only_one(&item.to.idents);
            let script = unwrap_only_one(&item.to.scripts);
            scripts_set.insert(script);
            let (amount_ckb, amount_udt) = if udt_hash.is_none() {
                (item.amount as u64, 0u128)
            } else {
                (0u64, item.amount)
            };

            let output_cells = self.build_outputs(
                &udt_hash,
                &parse_address(&addr)?,
                amount_ckb,
                amount_udt,
                &script,
                &mut amounts,
                from.idents[0].clone(),
                &mut output_capacity,
            )?;

            details_split_off(output_cells, &mut outputs, &mut cell_data);
        }

        let (inputs, consume, mut sigs_entry) =
            self.build_inputs(&udt_hash, from, &amounts, fee, &mut outputs, &mut cell_data)?;
        let (change_cell, change_data) = self.build_change_cell(
            change,
            udt_hash,
            output_capacity - consume.ckb - fee,
            amounts.udt_amount - consume.udt,
        )?;
        let cell_deps = self.build_cell_deps(scripts_set);

        outputs.push(change_cell);
        cell_data.push(change_data);

        let view = self.build_tx_view(cell_deps, inputs, outputs, cell_data);
        let tx_hash = view.hash().raw_data();
        sigs_entry
            .iter_mut()
            .for_each(|entry| entry.message = tx_hash.clone());

        Ok(TransferCompletionResponse::new(view.into(), sigs_entry))
    }

    fn build_tx_view(
        &self,
        deps: Vec<packed::CellDep>,
        inputs: Vec<packed::OutPoint>,
        outputs: Vec<packed::CellOutput>,
        data: Vec<packed::Bytes>,
    ) -> TransactionView {
        let since: packed::Uint64 = 0u64.pack();

        TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .cell_deps(deps)
            .inputs(inputs.into_iter().map(|input| {
                packed::CellInputBuilder::default()
                    .since(since.clone())
                    .previous_output(input)
                    .build()
            }))
            .outputs(outputs)
            .outputs_data(data)
            .build()
    }

    fn build_inputs(
        &self,
        udt_hash: &Option<H256>,
        from: InnerAccount,
        amounts: &DetailedAmount,
        fee: u64,
        outputs: &mut Vec<packed::CellOutput>,
        output_data: &mut Vec<packed::Bytes>,
    ) -> Result<(Vec<packed::OutPoint>, InputConsume, Vec<SignatureEntry>)> {
        let mut ckb_needed = BigUint::from(amounts.ckb_all + fee + MIN_CKB_CAPACITY);
        let mut udt_needed = BigUint::from(amounts.udt_amount);
        let (mut inputs, mut acp_outputs, mut sigs_entry) = (vec![], vec![], vec![]);
        let (mut capacity_sum, mut udt_sum_except_acp) = (0u64, 0u128);

        // Todo: can refactor here.
        if udt_needed.is_zero() {
            // An CkB transfer transaction.
            for ident in from.idents.iter() {
                let addr = Address::from_str(ident).map_err(MercuryError::ParseCKBAddressError)?;
                let script = address_to_script(addr.payload());
                let (cells, out_points) = self.get_cells_by_lock_script(&script)?;
                let sp_cells = self.get_sp_cells_by_addr(&addr)?.inner();
                let acps_by_from = self.take_sp_cells(&sp_cells, special_cells::ACP);
                let ckb_iter = ckb_iter(&cells, &out_points);

                self.pool_acp(
                    acps_by_from,
                    &mut ckb_needed,
                    &mut udt_needed,
                    &mut inputs,
                    &mut acp_outputs,
                    &mut capacity_sum,
                )?;

                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    &mut inputs,
                    &mut sigs_entry,
                    &mut capacity_sum,
                );
            }
        } else {
            // An UDT transfer transaction.
            let udt_hash = udt_hash.clone().unwrap();

            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let script = address_to_script(addr.payload());
                let (cells, out_points) = self.get_cells_by_lock_script(&script)?;
                let ckb_iter = ckb_iter(&cells, &out_points);
                let udt_iter = udt_iter(&cells, &out_points, udt_hash.pack());
                let sp_cells = self.get_sp_cells_by_addr(&addr)?.inner();
                let acps_by_from = self.take_sp_cells(&sp_cells, special_cells::ACP);

                self.pool_acp(
                    acps_by_from,
                    &mut ckb_needed,
                    &mut udt_needed,
                    &mut inputs,
                    &mut acp_outputs,
                    &mut capacity_sum,
                )?;

                self.pool_udt(
                    udt_iter,
                    &mut udt_needed,
                    &mut inputs,
                    &mut capacity_sum,
                    &mut udt_sum_except_acp,
                );

                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    &mut inputs,
                    &mut sigs_entry,
                    &mut capacity_sum,
                );
            }

            details_split_off(acp_outputs, outputs, output_data);

            // Todo: can do perf here.
            if let Some(tmp) = (*ACP_USED_CACHE).get(&thread::current().id()) {
                let (mut acp_used, ckb_used) = tmp.clone();
                inputs.append(&mut acp_used);
                capacity_sum += ckb_used;
            }
        }

        Ok((
            inputs,
            InputConsume::new(capacity_sum, udt_sum_except_acp),
            sigs_entry,
        ))
    }

    fn build_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        ckb_amount: u64,
        udt_amount: u128,
        script: &ScriptType,
        amounts: &mut DetailedAmount,
        from_addr: String,
        capacity_sum: &mut u64,
    ) -> Result<Vec<CelllWithData>> {
        if script.is_acp() {
            return self.build_acp_outputs(
                udt_hash,
                to_addr,
                from_addr,
                udt_amount,
                amounts,
                capacity_sum,
            );
        }

        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_amount, amounts)?;
        let lock_script = self.build_lock_script(to_addr, script, from_addr)?;
        let cell = packed::CellOutputBuilder::default()
            .lock(lock_script)
            .type_(type_script.pack())
            .build();
        let mut capacity: u64 = cell.capacity().unpack();

        if udt_hash.is_none() {
            capacity += ckb_amount;
            amounts.add_ckb_all(ckb_amount);
        } else {
            capacity += (data.len() as u64) * BYTE_SHANNONS;
            self.add_detailed_amount(amounts, to_addr.to_string(), capacity, script);
        }

        *capacity_sum += capacity;

        Ok(vec![CelllWithData::new(
            cell.as_builder().capacity(capacity.pack()).build(),
            data,
        )])
    }

    fn build_type_script(
        &self,
        udt_hash: Option<H256>,
        amount: u128,
        amounts: &mut DetailedAmount,
    ) -> Result<(Option<packed::Script>, Bytes)> {
        if let Some(hash) = udt_hash {
            let byte32 = hash.pack();
            let key = udt_balance::Key::ScriptHash(&byte32);
            let mut script_bytes = self
                .store_get(*UDT_EXT_PREFIX, key.into_vec())?
                .ok_or_else(|| MercuryError::UDTInexistence(hex::encode(hash.as_bytes())))?;
            let _is_sudt = script_bytes.remove(0) == 1;
            let script = packed::Script::from_slice(&script_bytes).unwrap();
            let data = Bytes::from(amount.to_le_bytes().to_vec());
            amounts.add_udt_amount(amount);

            Ok((Some(script), data))
        } else {
            Ok((None, Default::default()))
        }
    }

    fn build_lock_script(
        &self,
        to_addr: &Address,
        script: &ScriptType,
        from_addr: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();

        let script: packed::Script = match script {
            ScriptType::Secp256k1 => to_addr.payload().into(),
            ScriptType::Cheque => {
                let code_hash = self.config.get(CHEQUE).unwrap().script.code_hash();
                let receiver_lock: packed::Script = to_addr.payload().into();
                let sender_lock: packed::Script = parse_address(&from_addr)?.payload().into();
                let mut lock_args = Vec::from(&receiver_lock.calc_script_hash().as_slice()[0..20]);
                lock_args.extend_from_slice(&sender_lock.calc_script_hash().as_slice()[0..20]);

                script_builder
                    .code_hash(code_hash)
                    .hash_type(ScriptHashType::Type.into())
                    .args(lock_args.pack())
                    .build()
            }
            _ => unreachable!(),
        };

        Ok(script)
    }

    fn build_acp_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        from_addr: String,
        amount: u128,
        amounts: &mut DetailedAmount,
        capacity_sum: &mut u64,
    ) -> Result<Vec<CelllWithData>> {
        let mut ret = self.build_outputs(
            udt_hash,
            to_addr,
            0u64,
            amount,
            &ScriptType::Secp256k1,
            amounts,
            from_addr,
            capacity_sum,
        )?;

        let ckb_needed: u64 = ret[0].cell.capacity().unpack();
        let mut capacity_needed = BigUint::from(ckb_needed);
        let sp_cells = self.get_sp_cells_by_addr(to_addr)?.inner();
        let acp_cells = self.take_sp_cells(&sp_cells, special_cells::ACP);
        let (mut acp_used, mut acp_outputs, mut acp_capacity_sum) = (vec![], vec![], 0);

        self.pool_acp(
            acp_cells,
            &mut capacity_needed,
            &mut Zero::zero(),
            &mut acp_used,
            &mut acp_outputs,
            &mut acp_capacity_sum,
        )?;

        if capacity_needed > Zero::zero() {
            return Err(MercuryError::LackACPCells(to_addr.to_string()).into());
        }

        ret.append(&mut acp_outputs);
        *capacity_sum += acp_capacity_sum;
        ACP_USED_CACHE.insert(thread::current().id(), (acp_used, acp_capacity_sum));

        Ok(ret)
    }

    // Todo: have question here.
    fn build_change_cell(
        &self,
        addr: String,
        udt_hash: Option<H256>,
        ckb_change: u64,
        udt_change: u128,
    ) -> Result<(packed::CellOutput, packed::Bytes)> {
        let address = parse_address(&addr)?;
        let (type_script, data) =
            self.build_type_script(udt_hash, udt_change, &mut Default::default())?;
        let lock_script =
            self.build_lock_script(&address, &ScriptType::Secp256k1, Default::default())?;

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(type_script.pack())
                .capacity(ckb_change.pack())
                .build(),
            data.pack(),
        ))
    }

    fn ckb_balance(&self, addr: &Address) -> Result<Option<u64>> {
        let addr_string = addr.to_string();
        let key = ckb_balance::Key::CkbAddress(&addr_string);

        let raw = self.store_get(*CKB_EXT_PREFIX, key.into_vec())?;
        Ok(raw.map(|bytes| u64::from_be_bytes(to_fixed_array(&bytes))))
    }

    fn udt_balance(&self, addr: &Address, udt_hash: H256) -> Result<Option<u128>> {
        let mut encoded = udt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&addr.to_string().as_bytes());
        let key = udt_balance::Key::Address(&encoded);

        let raw = self.store_get(*UDT_EXT_PREFIX, key.into_vec())?;
        Ok(raw.map(|bytes| u128::from_be_bytes(to_fixed_array(&bytes))))
    }

    fn add_detailed_amount(
        &self,
        amounts: &mut DetailedAmount,
        to_addr: String,
        capacity: u64,
        script_type: &ScriptType,
    ) {
        match script_type {
            ScriptType::Secp256k1 => amounts.add_ckb_by_owned(capacity),
            ScriptType::AnyoneCanPay => amounts.add_ckb_by_acp(to_addr, capacity),
            ScriptType::Cheque => amounts.add_ckb_lend(capacity),
            _ => unreachable!(),
        };
    }

    fn pool_acp(
        &self,
        acp_cells: Vec<DetailedCell>,
        ckb_needed: &mut BigUint,
        sudt_needed: &mut BigUint,
        acp_used: &mut Vec<packed::OutPoint>,
        acp_outputs: &mut Vec<CelllWithData>,
        capacity_sum: &mut u64,
    ) -> Result<()> {
        for detail in acp_cells {
            if ckb_needed.is_zero() && sudt_needed.is_zero() {
                break;
            }

            let (consumable, base) = capacity_detail(&detail)?;
            let acp_data = detail.cell_data.raw_data().to_vec();
            let sudt_amount = decode_udt_amount(&acp_data);

            let capacity = u64_sub(consumable, ckb_needed.clone()) + base;
            let cell = packed::CellOutputBuilder::default()
                .type_(detail.cell_output.type_())
                .lock(detail.cell_output.lock())
                .capacity(capacity.pack())
                .build();

            let mut cell_data = encode_udt_amount(u128_sub(sudt_amount, sudt_needed.clone()));
            cell_data.extend_from_slice(&acp_data[16..]);

            acp_outputs.push(CelllWithData::new(cell, Bytes::from(cell_data)));
            acp_used.push(detail.out_point);
            *capacity_sum += capacity;

            *ckb_needed -= consumable.min(ckb_needed.clone().try_into().unwrap());
            *sudt_needed -= sudt_amount.min(sudt_needed.clone().try_into().unwrap());
        }

        Ok(())
    }

    fn pool_ckb<'a, I: Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)>>(
        &self,
        ckb_iter: I,
        ckb_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        sigs_entry: &mut Vec<SignatureEntry>,
        capacity_sum: &mut u64,
    ) {
        let mut sig_entry = Vec::new();

        for (ckb_cell, out_point) in ckb_iter {
            if ckb_needed.is_zero() {
                break;
            }

            let capacity: u64 = ckb_cell.cell_output.capacity().unpack();
            let consume_ckb = capacity.min(ckb_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            if sig_entry.is_empty() {
                sig_entry.push(SignatureEntry {
                    index: inputs.len() - 1,
                    type_: WitnessType::WitnessArgsLock,
                    message: Default::default(),
                    pub_key: H160::from_slice(&ckb_cell.cell_output.lock().args().raw_data())
                        .unwrap(),
                });
            }

            *ckb_needed -= consume_ckb;
            *capacity_sum += consume_ckb;
        }

        sigs_entry.append(&mut sig_entry);
    }

    fn pool_udt<'a, I: Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)>>(
        &self,
        udt_iter: I,
        udt_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        capacity_sum: &mut u64,
        udt_sum: &mut u128,
    ) {
        for (udt_cell, out_point) in udt_iter {
            if udt_needed.is_zero() {
                break;
            }

            let capacity: u64 = udt_cell.cell_output.capacity().unpack();
            let amount = decode_udt_amount(&udt_cell.cell_data.raw_data().to_vec());
            let udt_used = amount.min(udt_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            *udt_needed -= udt_used;
            *udt_sum += udt_used;
            *capacity_sum += capacity;
        }
    }

    fn store_get<P: AsRef<[u8]>, K: AsRef<[u8]>>(
        &self,
        prefix: P,
        key: K,
    ) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(prefix, key)).map_err(Into::into)
    }

    fn get_sp_cells_by_addr(&self, addr: &Address) -> Result<DetailedCells> {
        let args = H160::from_slice(&addr.payload().args().as_ref()[0..20]).unwrap();
        let key = special_cells::Key::CkbAddress(&args);
        let bytes = self
            .store_get(*SP_CELL_EXT_PREFIX, key.into_vec())?
            .ok_or_else(|| MercuryError::NoACPInThisAddress(addr.to_string()))?;
        let ret = deserialize::<DetailedCells>(&bytes).unwrap();
        if ret.is_empty() {
            return Err(MercuryError::NoACPInThisAddress(addr.to_string()).into());
        }

        Ok(ret)
    }

    fn take_sp_cells(&self, cell_list: &[DetailedCell], cell_name: &str) -> Vec<DetailedCell> {
        let script_code_hash = self.config.get(cell_name).unwrap().script.code_hash();
        cell_list
            .iter()
            .filter(|cell| cell.cell_output.lock().code_hash() == script_code_hash)
            .cloned()
            .collect()
    }

    fn get_cells_by_lock_script(
        &self,
        lock_script: &packed::Script,
    ) -> Result<(Vec<DetailedLiveCell>, Vec<packed::OutPoint>)> {
        let mut cells = Vec::new();
        let mut ret: Vec<packed::OutPoint> = Vec::new();
        let out_points =
            self.get_cells_by_script(lock_script, indexer::KeyPrefix::CellLockScript)?;

        for out_point in out_points.iter() {
            let cell = self.get_detailed_live_cell(out_point)?.ok_or_else(|| {
                MercuryError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                }
            })?;

            cells.push(cell);
            ret.push(out_point.clone());
        }

        Ok((cells, ret))
    }

    fn get_cells_by_script(
        &self,
        script: &packed::Script,
        prefix: indexer::KeyPrefix,
    ) -> Result<Vec<packed::OutPoint>> {
        let mut start_key = vec![prefix as u8];
        start_key.extend_from_slice(&extract_raw_data(script));
        let iter = self.store.iter(&start_key, IteratorDirection::Forward)?;

        Ok(iter
            .take_while(|(key, _)| key.starts_with(&start_key))
            .map(|(key, value)| {
                let tx_hash = packed::Byte32::from_slice(&value).expect("stored tx hash");
                let index = OutputIndex::from_be_bytes(
                    key[key.len() - 4..].try_into().expect("stored index"),
                );
                packed::OutPoint::new(tx_hash, index)
            })
            .collect())
    }

    fn get_detailed_live_cell(
        &self,
        out_point: &packed::OutPoint,
    ) -> Result<Option<DetailedLiveCell>> {
        let key_vec = indexer::Key::OutPoint(&out_point).into_vec();
        let (block_number, tx_index, cell_output, cell_data) = match self.store.get(&key_vec)? {
            Some(stored_cell) => indexer::Value::parse_cell_value(&stored_cell),
            None => return Ok(None),
        };
        let mut header_start_key = vec![indexer::KeyPrefix::Header as u8];
        header_start_key.extend_from_slice(&block_number.to_be_bytes());
        let mut iter = self
            .store
            .iter(&header_start_key, IteratorDirection::Forward)?;
        let block_hash = match iter.next() {
            Some((key, _)) => {
                if key.starts_with(&header_start_key) {
                    let start = std::mem::size_of::<BlockNumber>() + 1;
                    packed::Byte32::from_slice(&key[start..start + 32])
                        .expect("stored key header hash")
                } else {
                    return Ok(None);
                }
            }
            None => return Ok(None),
        };

        Ok(Some(DetailedLiveCell {
            block_number,
            block_hash,
            tx_index,
            cell_output,
            cell_data,
        }))
    }

    fn build_cell_deps(&self, scripts_set: HashSet<ScriptType>) -> Vec<packed::CellDep> {
        scripts_set
            .into_iter()
            .map(|s| self.config.get(s.as_str()).cloned().unwrap().cell_dep)
            .collect()
    }
}

fn address_to_script(payload: &AddressPayload) -> packed::Script {
    packed::ScriptBuilder::default()
        .code_hash(payload.code_hash())
        .hash_type(payload.hash_type().into())
        .args(payload.args().pack())
        .build()
}

fn udt_iter<'a>(
    input: &'a [DetailedLiveCell],
    out_points: &'a [packed::OutPoint],
    hash: packed::Byte32,
) -> impl Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)> {
    input
        .iter()
        .zip(out_points.iter())
        .filter(move |(cell, _)| {
            if let Some(script) = cell.cell_output.type_().to_opt() {
                script.calc_script_hash() == hash
            } else {
                false
            }
        })
}

fn ckb_iter<'a>(
    cells: &'a [DetailedLiveCell],
    out_points: &'a [packed::OutPoint],
) -> impl Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)> {
    cells
        .iter()
        .zip(out_points.iter())
        .filter(|(cell, _)| cell.cell_output.type_().is_none())
}

fn capacity_detail(cell: &DetailedCell) -> Result<(u64, u64)> {
    let capacity: u64 = cell.cell_output.capacity().unpack();
    let base = cell
        .cell_output
        .occupied_capacity(Capacity::shannons(
            (cell.cell_data.len() as u64) * BYTE_SHANNONS,
        ))?
        .as_u64();

    Ok((capacity - base, base))
}
