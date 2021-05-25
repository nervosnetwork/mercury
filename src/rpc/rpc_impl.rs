use crate::extensions::{
    ckb_balance, rce_validator, udt_balance, ACP_EXT_PREFIX, CKB_EXT_PREFIX, RCE_EXT_PREFIX,
    UDT_EXT_PREFIX,
};
use crate::rpc::types::{
    DetailedAmount, DetailedCell, InnerAccount, InnerTransferItem, ScriptType,
    TransferCompletionResponse, TransferPayload,
};
use crate::stores::add_prefix;
use crate::types::{DeployedScriptConfig, HashType};
use crate::utils::{parse_address, to_fixed_array};
use crate::{error::MercuryError, rpc::MercuryRpc};

use anyhow::Result;
use ckb_indexer::indexer::{self, extract_raw_data, OutputIndex};
use ckb_indexer::store::{IteratorDirection, Store};
use ckb_sdk::{Address, AddressPayload, NetworkType};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use jsonrpc_core::{Error, Result as RpcResult};

use std::collections::HashMap;
use std::convert::TryInto;

const ACP: &str = "anyone_can_pay";
const CHEQUE: &str = "cheque";
const SHANNON: u64 = 100_000_000;

macro_rules! rpc_try {
    ($input: expr) => {
        $input.map_err(|e| Error::invalid_params(e.to_string()))?
    };
}

pub struct MercuryRpcImpl<S> {
    store: S,
    network_ty: NetworkType,
    config: HashMap<String, DeployedScriptConfig>,
}

impl<S> MercuryRpc for MercuryRpcImpl<S>
where
    S: Store + Send + Sync + 'static,
{
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.ckb_balance(address));
        Ok(ret)
    }

    fn get_sudt_balance(&self, sudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(address, sudt_hash));
        Ok(ret)
    }

    fn get_xudt_balance(&self, xudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = rpc_try!(parse_address(&addr));
        let ret = rpc_try!(self.udt_balance(address, xudt_hash));
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
    pub fn new(
        store: S,
        network_ty: NetworkType,
        config: HashMap<String, DeployedScriptConfig>,
    ) -> Self {
        MercuryRpcImpl {
            store,
            network_ty,
            config,
        }
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

            let output_cell = self.build_output(
                None,
                addr,
                item.amount as u64,
                0,
                script,
                &mut amounts,
                from.idents[0].clone(),
            )?;

            outputs.push(output_cell.cell);
            cell_data.push(output_cell.data);
        }

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
    ) -> Result<Vec<packed::OutPoint>> {
        let ckb_amount = amounts.ckb_all + fee;
        let udt_amount = amounts.udt_amount;
        //let mut inputs = Vec::new();

        if udt_amount != 0 {
            todo!()
        }

        for ident in from.idents.iter() {}

        Ok(Default::default())
    }

    fn build_output(
        &self,
        udt_hash: Option<H256>,
        addr: String,
        ckb_amount: u64,
        udt_amount: u128,
        script: &ScriptType,
        amounts: &mut DetailedAmount,
        from_addr: String,
    ) -> Result<DetailedCell> {
        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_amount, amounts)?;
        let lock_script = self.build_lock_script(addr.clone(), script, from_addr)?;
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
            self.add_detailed_amount(amounts, addr, capacity, script);
        }

        let cell = cell.as_builder().capacity(capacity.pack()).build();

        Ok(DetailedCell::new(cell, data))
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
                .ok_or(MercuryError::UDTInexistence(hex::encode(hash.as_bytes())))?;
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
        addr: String,
        script: &ScriptType,
        from_address: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();
        let addr = parse_address(addr.as_str())?;
        let script: packed::Script = match script {
            ScriptType::Secp256k1 => addr.payload().into(),
            ScriptType::AnyoneCanPay => {
                // Todo: change this when #20 merge.
                let key = H160::from_slice(&addr.payload().args().as_ref()[0..20]).unwrap();
                let _acp_cells = self
                    .store_get(*ACP_EXT_PREFIX, key.as_bytes())?
                    .ok_or(MercuryError::NoACPInThisAddress(addr.to_string()))?;
                todo!()
            }
            ScriptType::Cheque => {
                let code_hash = self.config.get(CHEQUE).unwrap().script.code_hash();
                let receiver_lock: packed::Script = addr.payload().into();
                let sender_lock: packed::Script = parse_address(&from_address)?.payload().into();

                let mut lock_args = Vec::from(&receiver_lock.calc_script_hash().as_slice()[0..20]);
                lock_args.extend_from_slice(&sender_lock.calc_script_hash().as_slice()[0..20]);
                script_builder
                    .code_hash(code_hash)
                    .hash_type(HashType::Type.into())
                    .args(lock_args.pack())
                    .build()
            }
            _ => unreachable!(),
        };

        Ok(script)
    }

    fn get_balance(
        &self,
        udt_hash: Option<H256>,
        lock_script: &packed::Script,
    ) -> Result<(u64, u128)> {
        let addr = Address::new(self.network_ty, AddressPayload::from(lock_script.clone()));

        let ckb_balance = self.ckb_balance(addr)?.unwrap_or(0);

        todo!()
    }

    fn ckb_balance(&self, addr: Address) -> Result<Option<u64>> {
        let addr_string = addr.to_string();
        let key = ckb_balance::Key::CkbAddress(&addr_string);

        let raw = self.store_get(*CKB_EXT_PREFIX, key.into_vec())?;
        Ok(raw.and_then(|bytes| Some(u64::from_be_bytes(to_fixed_array(&bytes)))))
    }

    fn udt_balance(&self, addr: Address, udt_hash: H256) -> Result<Option<u128>> {
        let mut encoded = udt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&addr.to_string().as_bytes());
        let key = udt_balance::Key::Address(&encoded);

        let raw = self.store_get(*UDT_EXT_PREFIX, key.into_vec())?;
        Ok(raw.and_then(|bytes| Some(u128::from_be_bytes(to_fixed_array(&bytes)))))
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

    fn store_get<P: AsRef<[u8]>, K: AsRef<[u8]>>(
        &self,
        prefix: P,
        key: K,
    ) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(prefix, key)).map_err(Into::into)
    }

    fn get_live_cells_by_lock_script(
        &self,
        lock_script: &packed::Script,
    ) -> Result<Vec<packed::OutPoint>> {
        self.get_cells_by_script(lock_script, indexer::KeyPrefix::CellLockScript)
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
}
