use crate::{error::RpcErrorMessage, CkbRpc};

use common::{utils::to_fixed_array, MercuryError, Result};
use core_synchronization::SyncAdapter;

use async_trait::async_trait;
use ckb_jsonrpc_types::{
    BlockView, EpochView, JsonBytes, LocalNode, RawTxPool, TransactionWithStatus, Uint32, Uint64,
};
use ckb_types::{core, core::BlockNumber, packed, prelude::Entity, H256};
use jsonrpc_core::types::{
    Call, Id, MethodCall, Output, Params, Request, Response, Value, Version,
};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const LOCAL_NODE_INFO_REQ: &str = "local_node_info";
const GET_RAW_TX_POOL_REQ: &str = "get_raw_tx_pool";
const GET_TRANSACTION_REQ: &str = "get_transaction";
const GET_BLOCK_REQ: &str = "get_block";
const GET_BLOCK_BY_NUMBER_REQ: &str = "get_block_by_number";
const GET_TIP_BLOCK_NUMBER_REQ: &str = "get_tip_block_number";
const GET_EPOCH_BY_NUMBER_REQ: &str = "get_epoch_by_number";

#[derive(Clone, Debug)]
pub struct CkbRpcClient {
    ckb_uri: String,
    req_builder: RequestBuilder,
}

#[async_trait]
impl SyncAdapter for CkbRpcClient {
    async fn pull_blocks(&self, block_numbers: Vec<BlockNumber>) -> Result<Vec<core::BlockView>> {
        let mut ret = Vec::new();
        for block in self.get_blocks_by_number(block_numbers).await?.iter() {
            if let Some(b) = block {
                ret.push(core::BlockView::from(b.to_owned()));
            } else {
                return Err(RpcErrorMessage::GetNoneBlockFromNode.into());
            }
        }

        Ok(ret)
    }
}

#[async_trait]
impl CkbRpc for CkbRpcClient {
    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool> {
        let (id, request) = self.build_request(GET_RAW_TX_POOL_REQ, vec![verbose])?;
        let resp = self.rpc_exec(&request, id).await?;
        handle_response(resp)
    }

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }

        let (id, request) = self.build_batch_request(GET_TRANSACTION_REQ, hashes)?;
        let resp = self.rpc_exec(&request, id).await?;
        handle_batch_response(resp)
    }

    async fn get_blocks_by_number(
        &self,
        block_number: Vec<BlockNumber>,
    ) -> Result<Vec<Option<BlockView>>> {
        if block_number.is_empty() {
            return Ok(Vec::new());
        }

        let block_number: Vec<Uint64> = block_number.into_iter().map(Into::into).collect();
        let (id, request) = self.build_batch_request(GET_BLOCK_BY_NUMBER_REQ, block_number)?;
        let resp = self.rpc_exec(&request, id).await?;

        handle_batch_response(resp)
    }

    async fn get_epoch_by_number(&self, epoch_number: u64) -> Result<EpochView> {
        let (id, request) = self.build_request(GET_EPOCH_BY_NUMBER_REQ, vec![epoch_number])?;
        let resp = self.rpc_exec(&request, id).await?;
        handle_response(resp)
    }

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>> {
        let (id, request) = if use_hex_format {
            let verbose: Uint32 = 0u32.into();
            self.build_request(GET_BLOCK_REQ, (block_hash, Some(verbose)))?
        } else {
            self.build_request(GET_BLOCK_REQ, vec![block_hash])?
        };

        let resp = self.rpc_exec(&request, id).await?;

        if use_hex_format {
            let ret = handle_response::<Option<JsonBytes>>(resp)?;
            Ok(ret.map(|json_bytes| {
                packed::Block::new_unchecked(json_bytes.into_bytes())
                    .into_view()
                    .into()
            }))
        } else {
            handle_response(resp)
        }
    }

    async fn get_tip_block_number(&self) -> Result<BlockNumber> {
        let (id, request) = self.build_request(GET_TIP_BLOCK_NUMBER_REQ, ())?;
        let resp = self.rpc_exec(&request, id).await?;
        let ret: Uint64 = handle_response(resp)?;
        Ok(ret.into())
    }

    async fn local_node_info(&self) -> Result<LocalNode> {
        let (id, request) = self.build_request(LOCAL_NODE_INFO_REQ, ())?;
        let resp = self.rpc_exec(&request, id).await?;
        handle_response(resp)
    }
}

impl CkbRpcClient {
    pub fn new(uri: String) -> Self {
        CkbRpcClient {
            ckb_uri: uri,
            req_builder: RequestBuilder::new(),
        }
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

    pub async fn relay_exec(&self, request: Request, id: Id) -> Option<Response> {
        self.rpc_exec(&request, id).await.ok()
    }

    async fn rpc_exec(&self, request: &Request, id: Id) -> Result<Response> {
        log::debug!(
            "sending request {:?}, id {:?}",
            serde_json::to_string(&request)?,
            id
        );

        let http_response = Client::new()
            .post(self.ckb_uri.as_str())
            .json(request)
            .send()
            .await?;

        if !http_response.status().is_success() {
            return Err(MercuryError::rpc(RpcErrorMessage::CkbClientError(format!(
                "response status code is not success: {}",
                http_response.status()
            )))
            .into());
        }

        http_response
            .json()
            .await
            .map_err(|e| MercuryError::rpc(RpcErrorMessage::DecodeJson(e.to_string())).into())
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
                params: parse_params(&vec![item])?,
                id: id.clone(),
            }));
        }

        Ok((id, Request::Batch(calls)))
    }
}

fn parse_params<T: Serialize>(params: &T) -> Result<Params> {
    let json = serde_json::to_value(params).unwrap();

    match json {
        Value::Array(vec) => Ok(Params::Array(vec)),
        Value::Object(map) => Ok(Params::Map(map)),
        Value::Null => Ok(Params::None),
        _ => {
            Err(MercuryError::rpc(RpcErrorMessage::InvalidRpcParams("ckb rpc".to_string())).into())
        }
    }
}

fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
    match response {
        Response::Single(output) => handle_output(output),
        _ => unreachable!(),
    }
}

fn handle_batch_response<T: DeserializeOwned>(response: Response) -> Result<Vec<T>> {
    match response {
        Response::Batch(outputs) => {
            let mut ret = Vec::new();
            for output in outputs.into_iter() {
                let json = handle_output(output)?;
                ret.push(json)
            }
            Ok(ret)
        }
        _ => unreachable!(),
    }
}

fn handle_output<T: DeserializeOwned>(output: Output) -> Result<T> {
    let value = match output {
        Output::Success(succ) => succ.result,
        Output::Failure(fail) => {
            return Err(
                MercuryError::rpc(RpcErrorMessage::DecodeJson(fail.error.to_string())).into(),
            )
        }
    };

    serde_json::from_value(value)
        .map_err(|e| MercuryError::rpc(RpcErrorMessage::DecodeJson(e.to_string())).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpc_http_server::tokio;

    const CKB_URI: &str = "http://127.0.0.1:8114";

    #[tokio::test]
    #[ignore]
    async fn test_ckb_rpc_client() {
        let client = CkbRpcClient::new(CKB_URI.to_string());

        let res = client.get_raw_tx_pool(Some(true)).await.unwrap();
        println!("{:?}", res);

        let res = client.get_raw_tx_pool(Some(false)).await.unwrap();
        println!("{:?}", res);

        let res = client.get_raw_tx_pool(None).await.unwrap();
        println!("{:?}", res);

        let res = client.local_node_info().await.unwrap();
        println!("{:?}", res);

        let res = client.get_blocks_by_number(vec![895_654u64]).await.unwrap();
        assert!(res[0].is_some());
        println!("{:?}", res);

        let res = client.get_blocks_by_number(vec![895_654u64]).await.unwrap();
        assert!(res[0].is_some());
        println!("{:?}", res);

        let res = client.get_blocks_by_number(vec![u64::MAX]).await.unwrap();
        assert!(res[0].is_none());

        let res = client
            .get_transactions(vec![H256::from_trimmed_str(
                "98db47e087d93a4b0c784fbdd252c6e3fab9a62dbf8d553d0ecc6640b6f6c0c4",
            )
            .unwrap()])
            .await
            .unwrap();
        assert!(res[0].is_some());
        println!("{:?}", res);

        let res = client
            .get_transactions(vec![
                H256::from_trimmed_str(
                    "98db47e087d93a4b0c784fbdd252c6e3fab9a62dbf8d553d0ecc6640b6f6c0c4",
                )
                .unwrap(),
                H256::from_trimmed_str(
                    "725b20ed768f66008463b4cccceff31d34cbae4c040d8b44edd3277c533ff302",
                )
                .unwrap(),
            ])
            .await
            .unwrap();
        assert_eq!(res.len(), 2);

        let res = client
            .get_block(
                H256::from_trimmed_str(
                    "aea2ea99fcf74138533f6fd7681c6c44539a564ca6e415f1400d5beb09ebdec8",
                )
                .unwrap(),
                true,
            )
            .await
            .unwrap();
        println!("{:?}", res);

        let res = client
            .get_block(
                H256::from_trimmed_str(
                    "aea2ea99fcf74138533f6fd7681c6c44539a564ca6e415f1410d5beb09ebdec8",
                )
                .unwrap(),
                true,
            )
            .await
            .unwrap();
        assert!(res.is_none());

        let res = client.get_tip_block_number().await.unwrap();
        println!("{:?}", res);
    }
}
