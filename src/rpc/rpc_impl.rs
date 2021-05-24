use crate::extensions::{
    ckb_balance, rce_validator, udt_balance, CKB_EXT_PREFIX, RCE_EXT_PREFIX, UDT_EXT_PREFIX,
};
use crate::rpc::types::{
    DetailedCell, InnerAccount, InnerTransferItem, ScriptType, TransferCompletionResponse,
    TransferPayload,
};
use crate::stores::add_prefix;
use crate::types::{DeployedScriptConfig, HashType};
use crate::utils::{parse_address, to_fixed_array};
use crate::{error::MercuryError, rpc::MercuryRpc};

use anyhow::Result;
use ckb_indexer::store::Store;
use ckb_types::{packed, prelude::*, H256};
use jsonrpc_core::{Error, Result as RpcResult};

use std::collections::HashMap;

const ACP: &str = "anyone_can_pay";
const CHEQUE: &str = "cheque";

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
        let address = parse_address(&addr).map_err(|e| Error::invalid_params(e.to_string()))?;
        let key: Vec<u8> = ckb_balance::Key::CkbAddress(&address.to_string()).into();

        rpc_try!(self.store.get(&add_prefix(*CKB_EXT_PREFIX, key))).map_or_else(
            || Ok(None),
            |bytes| Ok(Some(u64::from_be_bytes(to_fixed_array(&bytes)))),
        )
    }

    fn get_sudt_balance(&self, sudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = parse_address(&addr).map_err(|e| Error::invalid_params(e.to_string()))?;
        let mut encoded = sudt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&address.to_string().as_bytes());
        let key: Vec<u8> = udt_balance::Key::Address(&encoded).into();

        rpc_try!(self.store.get(&add_prefix(*UDT_EXT_PREFIX, key))).map_or_else(
            || Ok(None),
            |bytes| Ok(Some(u128::from_be_bytes(to_fixed_array(&bytes)))),
        )
    }

    fn get_xudt_balance(&self, xudt_hash: H256, addr: String) -> RpcResult<Option<u128>> {
        let address = parse_address(&addr).map_err(|e| Error::invalid_params(e.to_string()))?;
        let mut encoded = xudt_hash.as_bytes().to_vec();
        encoded.extend_from_slice(&address.to_string().as_bytes());
        let key: Vec<u8> = udt_balance::Key::Address(&encoded).into();

        rpc_try!(self.store.get(&add_prefix(*UDT_EXT_PREFIX, key))).map_or_else(
            || Ok(None),
            |bytes| Ok(Some(u128::from_be_bytes(to_fixed_array(&bytes)))),
        )
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
        //let mut outputs = Vec::new();
        let mut capacity = 0u128;

        for item in items.iter() {
            assert!(item.to.idents.len() == 1);
            assert!(item.to.scripts.len() == 1);

            let addr = item.to.idents.get(0).cloned().unwrap();
            let script = item.to.scripts.get(0).unwrap();

            match script {
                ScriptType::Secp256k1 => todo!(),
                ScriptType::AnyoneCanPay => todo!(),
                ScriptType::Cheque => todo!(),
                _ => unreachable!(),
            };
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

    fn build_output(
        &self,
        udt_hash: Option<H256>,
        addr: String,
        amount: u128,
        script: ScriptType,
        from_addr: String,
    ) -> Result<DetailedCell> {
        let type_script = self.build_type_script(udt_hash.clone())?;
        let lock_script = self.build_lock_script(addr, script, from_addr)?;
        let tmp = packed::CellOutputBuilder::default()
            .lock(lock_script)
            .type_(type_script.pack())
            .build();
        let capacity: u64 = tmp.capacity().unpack();
        let cell = tmp.as_builder().capacity(capacity.pack()).build();

        Ok(Default::default())
    }

    fn build_type_script(&self, udt_hash: Option<H256>) -> Result<Option<packed::Script>> {
        if let Some(hash) = udt_hash {
            let mut script_bytes = self
                .store_get(*UDT_EXT_PREFIX, hash.as_bytes())?
                .ok_or(MercuryError::UDTInexistence(hex::encode(hash.as_bytes())))?;
            let _is_sudt = script_bytes.remove(0) == 1;
            let script = packed::Script::from_slice(&script_bytes).unwrap();
            Ok(Some(script))
        } else {
            Ok(None)
        }
    }

    fn build_lock_script(
        &self,
        addr: String,
        script: ScriptType,
        from_address: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();
        let addr = parse_address(addr.as_str())?;
        let script: packed::Script = match script {
            ScriptType::Secp256k1 => addr.payload().into(),
            ScriptType::AnyoneCanPay => todo!(),
            ScriptType::Cheque => {
                let code_hash = self.config.get(CHEQUE).unwrap().script.code_hash();
                let receiver_lock: packed::Script = addr.payload().into();
                let sender_lock: packed::Script = parse_address(&from_address)?.payload().into();
                let mut lock_args = Vec::new();
                lock_args.extend_from_slice(&receiver_lock.calc_script_hash().as_slice()[0..20]);
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

    fn store_get<P: AsRef<[u8]>, K: AsRef<[u8]>>(
        &self,
        prefix: P,
        key: K,
    ) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(prefix, key)).map_err(Into::into)
    }
}
