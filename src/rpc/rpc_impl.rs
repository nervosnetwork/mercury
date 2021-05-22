use crate::extensions::{
    ckb_balance, rce_validator, udt_balance, CKB_EXT_PREFIX, RCE_EXT_PREFIX, UDT_EXT_PREFIX,
};
use crate::rpc::types::{
    InnerAccount, InnerTransferItem, ScriptType, TransferCompletionResponse, TransferPayload,
};
use crate::stores::add_prefix;
use crate::utils::{parse_address, to_fixed_array};
use crate::{error::MercuryError, rpc::MercuryRpc};
use crate::types::DeployedScriptConfig;

use anyhow::Result;
use ckb_indexer::store::Store;
use ckb_types::{packed, prelude::Pack, H256};
use jsonrpc_core::{Error, Result as RpcResult};

use std::collections::HashMap;

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
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
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
    ) -> Result<packed::CellOutput> {
        let type_script = if let Some(hash) = udt_hash {
            let code_hash = 
            let script = packed::ScriptBuilder::default().code_hash(v)
        } else {
            None
        };

    }
}
