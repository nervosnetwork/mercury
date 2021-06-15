use crate::error::MercuryError;

use anyhow::Result;
use ckb_jsonrpc_types::{BlockView, RawTxPool, TransactionWithStatus};
use ckb_types::core::BlockNumber;
use ckb_types::H256;
use jsonrpc_core::types::{Call, Id, MethodCall, Params, Request, Value, Version};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const GET_RAW_TX_POOL_REQ: &str = "get_raw_tx_pool";
const GET_TRANSACTION_REQ: &str = "get_transaction";
const GET_BLOCK_BY_NUMBER_REQ: &str = "get_block_by_number";

#[derive(Clone, Debug)]
pub struct CkbRpcClient {
    ckb_uri: String,
    req_builder: RequestBuilder,
}

impl CkbRpcClient {
    pub fn new(uri: String) -> Self {
        CkbRpcClient {
            ckb_uri: uri,
            req_builder: RequestBuilder::new(),
        }
    }

    // Todo: move these to Trait
    pub async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool> {
        let (id, request) = self.build_request(GET_RAW_TX_POOL_REQ, verbose)?;
        rpc_exec(self.ckb_uri.as_str(), &request, id).await
    }

    pub async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>> {
        let (id, request) = self.build_batch_request(GET_TRANSACTION_REQ, hashes)?;
        rpc_exec(self.ckb_uri.as_str(), &request, id).await
    }

    pub async fn get_block_by_number(
        &self,
        block_number: BlockNumber,
        use_hex_format: bool,
    ) -> Result<Option<BlockView>> {
        let (id, request) = if use_hex_format {
            self.build_request(GET_BLOCK_BY_NUMBER_REQ, (block_number, Some(0u32)))?
        } else {
            self.build_request(GET_BLOCK_BY_NUMBER_REQ, block_number)?
        };

        rpc_exec(self.ckb_uri.as_str(), &request, id).await
    }

    fn build_request<T: Serialize>(&self, method: &str, params: T) -> Result<(Id, Request)> {
        self.req_builder.request(method.to_string(), params)
    }

    fn build_batch_request<T: Serialize>(
        &self,
        method: &str,
        params: Vec<T>,
    ) -> Result<(Id, Request)> {
        self.req_builder.batch_request(method.to_string(), params)
    }
}

#[derive(Debug)]
struct RequestBuilder {
    id: Arc<AtomicU64>,
}

impl Clone for RequestBuilder {
    fn clone(&self) -> Self {
        let id_clone = Arc::clone(&self.id);
        RequestBuilder { id: id_clone }
    }
}

impl RequestBuilder {
    fn new() -> Self {
        RequestBuilder {
            id: Arc::new(AtomicU64::default()),
        }
    }

    fn next_id(&self) -> Id {
        Id::Num(self.id.fetch_add(1, Ordering::SeqCst))
    }

    fn request<T: Serialize>(&self, method: String, params: T) -> Result<(Id, Request)> {
        let id = self.next_id();
        let request = Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method,
            params: parse_params(&params)?,
            id: id.clone(),
        }));

        Ok((id, request))
    }

    fn batch_request<T: Serialize>(&self, method: String, params: Vec<T>) -> Result<(Id, Request)> {
        let id = self.next_id();
        let mut calls = Vec::new();

        for item in params.iter() {
            calls.push(Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: method.clone(),
                params: parse_params(item)?,
                id: id.clone(),
            }));
        }

        Ok((id, Request::Batch(calls)))
    }
}

async fn rpc_exec<T: DeserializeOwned>(url: &str, request: &Request, id: Id) -> Result<T> {
    log::debug!(
        "sending request {:?}, id {:?}",
        serde_json::to_string(&request)?,
        id
    );

    let response = Client::new().post(url).json(request).send().await?;

    let status = response.status();
    if !status.is_success() {
        return Err(MercuryError::CkbRpcError(format!(
            "response status code is not success: {}",
            status
        ))
        .into());
    }

    response
        .json()
        .await
        .map_err(|e| MercuryError::DecodeJson(e.to_string()).into())
}

fn parse_params<T: Serialize>(params: &T) -> Result<Params> {
    let json = serde_json::to_value(params).unwrap();

    match json {
        Value::Array(vec) => Ok(Params::Array(vec)),
        Value::Object(map) => Ok(Params::Map(map)),
        Value::Null => Ok(Params::None),
        _ => Err(MercuryError::InvalidRpcParams("ckb rpc".to_string()).into()),
    }
}
