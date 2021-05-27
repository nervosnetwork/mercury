use crate::extensions::{
    anyone_can_pay, ckb_balance, rce_validator, udt_balance, ACP_EXT_PREFIX, CKB_EXT_PREFIX,
    RCE_EXT_PREFIX, UDT_EXT_PREFIX,
};
use crate::rpc::types::{
    details_split_off, DetailedAmount, DetailedCell, InnerAccount, InnerTransferItem, ScriptType,
    TransferCompletionResponse, TransferPayload,
};
use crate::stores::add_prefix;
use crate::types::DeployedScriptConfig;
use crate::utils::{parse_address, sub, to_fixed_array};
use crate::{error::MercuryError, rpc::MercuryRpc};

use anyhow::Result;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_indexer::store::{IteratorDirection, Store};
use ckb_sdk::{Address, AddressPayload};
use ckb_types::core::{BlockNumber, Capacity, ScriptHashType};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use dashmap::DashMap;
use jsonrpc_core::{Error, Result as RpcResult};
use num_bigint::BigInt;
use num_traits::identities::Zero;

use std::collections::HashMap;
use std::convert::TryInto;
use std::thread::{self, ThreadId};
use std::{iter::Iterator, str::FromStr};

const CHEQUE: &str = "cheque";
const SHANNON: u64 = 100_000_000;

lazy_static::lazy_static! {
    static ref ACP_USED_CACHE: DashMap<ThreadId, Vec<DetailedLiveCell>> = DashMap::new();
    static ref ZERO: BigInt = BigInt::zero();
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
        self.tranfer_complete(
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

    fn tranfer_complete(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransferCompletionResponse> {
        if let Some(hash) = udt_hash {
            self.udt_complete(hash, from, items, change, fee)
        } else {
            self.ckb_complete(from, items, change, fee)
        }
    }

    fn ckb_complete(
        &self,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransferCompletionResponse> {
        //let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut cell_data = Vec::new();
        let mut amounts = DetailedAmount::new();

        for item in items.iter() {
            assert!(item.to.idents.len() == 1);
            assert!(item.to.scripts.len() == 1);
            let addr = item.to.idents.get(0).cloned().unwrap();
            let script = item.to.scripts.get(0).unwrap();

            let output_cells = self.build_outputs(
                None,
                &parse_address(&addr)?,
                item.amount as u64,
                0,
                script,
                &mut amounts,
                from.idents[0].clone(),
            )?;

            details_split_off(output_cells, &mut outputs, &mut cell_data);
        }

        let inputs = self.build_inputs(None, from, amounts, fee, &mut outputs, &mut cell_data)?;

        todo!()
    }

    fn udt_complete(
        &self,
        udt_hash: H256,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransferCompletionResponse> {
        todo!()
    }

    fn build_inputs(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        amounts: DetailedAmount,
        fee: u64,
        outputs: &mut Vec<packed::CellOutput>,
        output_data: &mut Vec<Bytes>,
    ) -> Result<Vec<packed::OutPoint>> {
        let ckb_amount = amounts.ckb_all + fee;
        let udt_amount = amounts.udt_amount;
        let (mut inputs, mut acp_outputs) = (vec![], vec![]);

        if udt_amount != 0 {
            // filled with udt cell
            let udt_hash = udt_hash.unwrap();
            let mut udt_needs = BigInt::from(amounts.udt_amount);
            let mut ckb_needed = BigInt::from(amounts.ckb_all);

            for ident in from.idents.iter() {
                // filled with owned ckb cell
                let addr = parse_address(ident)?;
                let script = address_to_script(addr.payload());
                let cells = self.get_cells_by_lock_script(&script)?;
                let ckb_iter = ckb_iter(&cells);
                let udt_iter = udt_iter(&cells, udt_hash.pack());
                let acps_by_from = self.get_acp_cells_by_addr(&addr)?.into_iter();

                for udt_cell in udt_iter {
                    if udt_needs <= *ZERO {
                        break;
                    }

                    let raw_data: Vec<u8> = udt_cell.cell_data.unpack();
                    let amount = u128::from_le_bytes(to_fixed_array(&raw_data[0..16]));
                    udt_needs -= amount;
                    inputs.push(udt_cell.clone());
                }

                if amounts.ckb_all != 0 {
                    for ckb_cell in ckb_iter {
                        if ckb_needed <= *ZERO {
                            break;
                        }

                        let capacity: u64 = ckb_cell.cell_output.capacity().unpack();
                        ckb_needed -= capacity;
                        inputs.push(ckb_cell.clone());
                    }

                    self.pool_acps(acps_by_from, &mut ckb_needed, &mut inputs, &mut acp_outputs)?;
                }
            }

            details_split_off(acp_outputs, outputs, output_data);

            // Todo: can do perf here
            if let Some(tmp) = (*ACP_USED_CACHE).get(&thread::current().id()) {
                let mut acp_used = tmp.clone();
                inputs.append(&mut acp_used);
            }
        } else {
            for ident in from.idents.iter() {
                let addr = Address::from_str(ident).map_err(MercuryError::ParseCKBAddressError)?;
                let script = address_to_script(addr.payload());
                let cells = self.get_cells_by_lock_script(&script)?;
            }
        }

        Ok(Default::default())
    }

    fn build_outputs(
        &self,
        udt_hash: Option<H256>,
        to_addr: &Address,
        ckb_amount: u64,
        udt_amount: u128,
        script: &ScriptType,
        amounts: &mut DetailedAmount,
        from_addr: String,
    ) -> Result<Vec<DetailedCell>> {
        if script.is_acp() {
            return self.build_acp_outputs(udt_hash, to_addr, from_addr, udt_amount, amounts);
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
            capacity += (data.len() as u64) * SHANNON;
            self.add_detailed_amount(amounts, to_addr.to_string(), capacity, script);
        }

        Ok(vec![DetailedCell::new(
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
        udt_hash: Option<H256>,
        to_addr: &Address,
        from_addr: String,
        amount: u128,
        amounts: &mut DetailedAmount,
    ) -> Result<Vec<DetailedCell>> {
        let mut ret = self.build_outputs(
            udt_hash,
            to_addr,
            0,
            amount,
            &ScriptType::Secp256k1,
            amounts,
            from_addr,
        )?;

        let ckb_needed: u64 = ret[0].cell.capacity().unpack();
        let mut capacity_needed = BigInt::from(ckb_needed);
        let acp_cells = self.get_acp_cells_by_addr(to_addr)?.into_iter();
        let (mut acp_used, mut acp_outputs) = (vec![], vec![]);

        self.pool_acps(
            acp_cells,
            &mut capacity_needed,
            &mut acp_used,
            &mut acp_outputs,
        )?;

        if capacity_needed > *ZERO {
            return Err(MercuryError::LackACPCells(to_addr.to_string()).into());
        }

        ret.append(&mut acp_outputs);

        ACP_USED_CACHE.insert(thread::current().id(), acp_used);

        Ok(ret)
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

    fn pool_acps(
        &self,
        acp_cells: packed::OutPointVecIterator,
        ckb_needed: &mut BigInt,
        acp_used: &mut Vec<DetailedLiveCell>,
        acp_outputs: &mut Vec<DetailedCell>,
    ) -> Result<()> {
        for cell in acp_cells {
            if *ckb_needed <= *ZERO {
                break;
            }

            let detail = self.get_detailed_live_cell(&cell)?.unwrap();
            let (consumable, base) = capacity_detail(&detail)?;

            let cell = packed::CellOutputBuilder::default()
                .type_(detail.cell_output.type_())
                .lock(detail.cell_output.lock())
                .capacity((sub(consumable, ckb_needed.clone()) + base).pack())
                .build();

            acp_outputs.push(DetailedCell::new(cell, detail.cell_data.unpack()));
            *ckb_needed -= consumable;
            acp_used.push(detail);
        }

        Ok(())
    }

    fn store_get<P: AsRef<[u8]>, K: AsRef<[u8]>>(
        &self,
        prefix: P,
        key: K,
    ) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(prefix, key)).map_err(Into::into)
    }

    fn get_acp_cells_by_addr(&self, addr: &Address) -> Result<packed::OutPointVec> {
        let args = H160::from_slice(&addr.payload().args().as_ref()[0..20]).unwrap();
        let key = anyone_can_pay::Key::CkbAddress(&args);
        let bytes = self
            .store_get(*ACP_EXT_PREFIX, key.into_vec())?
            .ok_or_else(|| MercuryError::NoACPInThisAddress(addr.to_string()))?;
        let ret = packed::OutPointVec::from_slice(&bytes).unwrap();
        if ret.is_empty() {
            return Err(MercuryError::NoACPInThisAddress(addr.to_string()).into());
        }

        Ok(ret)
    }

    fn get_cells_by_lock_script(
        &self,
        lock_script: &packed::Script,
    ) -> Result<Vec<DetailedLiveCell>> {
        let mut ret = Vec::new();
        let out_points =
            self.get_cells_by_script(lock_script, indexer::KeyPrefix::CellLockScript)?;

        for out_point in out_points.iter() {
            let cell = self.get_detailed_live_cell(out_point)?.ok_or_else(|| {
                MercuryError::CannotGetLiveCellByOutPoint {
                    tx_hash: hex::encode(out_point.tx_hash().as_slice()),
                    index: out_point.index().unpack(),
                }
            })?;

            ret.push(cell);
        }

        Ok(ret)
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
}

fn address_to_script(payload: &AddressPayload) -> packed::Script {
    packed::ScriptBuilder::default()
        .code_hash(payload.code_hash())
        .hash_type(payload.hash_type().into())
        .args(payload.args().pack())
        .build()
}

fn udt_iter(
    input: &[DetailedLiveCell],
    hash: packed::Byte32,
) -> impl Iterator<Item = &DetailedLiveCell> {
    input.iter().filter(move |cell| {
        if let Some(script) = cell.cell_output.type_().to_opt() {
            script.calc_script_hash() == hash
        } else {
            false
        }
    })
}

fn ckb_iter(input: &[DetailedLiveCell]) -> impl Iterator<Item = &DetailedLiveCell> {
    input
        .iter()
        .filter(|cell| cell.cell_output.type_().is_none())
}

fn capacity_detail(cell: &DetailedLiveCell) -> Result<(u64, u64)> {
    let capacity: u64 = cell.cell_output.capacity().unpack();
    let base = cell
        .cell_output
        .occupied_capacity(Capacity::shannons((cell.cell_data.len() as u64) * SHANNON))?
        .as_u64();

    Ok((capacity - base, base))
}
