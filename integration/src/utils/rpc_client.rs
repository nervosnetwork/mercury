use crate::const_definition::{RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::{EpochView, LocalNode, OutputsValidator, Transaction};
use ckb_types::H256;
use core_rpc_types::{
    BlockInfo, GetBalancePayload, GetBalanceResponse, GetBlockInfoPayload, MercuryInfo, SyncState,
    TransactionCompletionResponse, TransferPayload,
};
use jsonrpc_core::types::{
    Call, Id, MethodCall, Output, Params, Request, Response, Value, Version,
};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Serialize};

use std::thread::sleep;
use std::time::Duration;

pub struct CkbRpcClient {
    client: RpcClient,
}

impl CkbRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        CkbRpcClient { client }
    }

    pub fn local_node_info(&self) -> Result<LocalNode> {
        request(&self.client, "local_node_info", ())
    }

    pub fn get_current_epoch(&self) -> Result<EpochView> {
        request(&self.client, "get_current_epoch", ())
    }

    pub fn generate_block(&self) -> Result<H256> {
        request(&self.client, "generate_block", ())
    }

    pub fn send_transaction(
        &self,
        tx: Transaction,
        outputs_validator: OutputsValidator,
    ) -> Result<H256> {
        request(&self.client, "send_transaction", (tx, outputs_validator))
    }
}

pub struct MercuryRpcClient {
    client: RpcClient,
}

impl MercuryRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        MercuryRpcClient { client }
    }

    pub fn get_balance(&self, payload: GetBalancePayload) -> Result<GetBalanceResponse> {
        request(&self.client, "get_balance", vec![payload])
    }

    pub fn get_mercury_info(&self) -> Result<MercuryInfo> {
        request(&self.client, "get_mercury_info", ())
    }

    pub fn get_block_info(&self, block_hash: H256) -> Result<BlockInfo> {
        let payload = GetBlockInfoPayload {
            block_hash: Some(block_hash),
            block_number: None,
        };
        request(&self.client, "get_block_info", vec![payload])
    }

    pub fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_transfer_transaction", vec![payload])
    }

    pub fn wait_block(&self, block_hash: H256) -> Result<()> {
        for _try in 0..=RPC_TRY_COUNT {
            let response = self.get_block_info(block_hash.clone());
            if response.is_ok() {
                return Ok(());
            }
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
        return Err(anyhow!("wait block fail"));
    }

    pub fn wait_sync(&self) {
        loop {
            let sync_state: SyncState =
                request(&self.client, "get_sync_state", ()).expect("get_sync_state");
            if let SyncState::Serial(progress) = sync_state {
                println!("{:?}", progress);
                if progress.current == progress.target {
                    break;
                }
            }
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RpcClient {
    client: Client,
    uri: String,
}

impl RpcClient {
    pub(crate) fn new(uri: String) -> Self {
        RpcClient {
            client: Client::new(),
            uri,
        }
    }

    pub(crate) fn rpc_exec(&self, request: &Request) -> Result<Response> {
        let http_response = self.client.post(self.uri.as_str()).json(request).send()?;

        if !http_response.status().is_success() {
            return Err(anyhow!("http response"));
        }

        http_response.json().map_err(anyhow::Error::new)
    }
}

fn request<T: Serialize, U: DeserializeOwned>(
    client: &RpcClient,
    method: &str,
    params: T,
) -> Result<U> {
    let request = build_request(method, params)?;
    let response = client.rpc_exec(&request)?;
    handle_response(response)
}

fn build_request<T: Serialize>(method: &str, params: T) -> Result<Request> {
    let request = Request::Single(Call::MethodCall(MethodCall {
        jsonrpc: Some(Version::V2),
        method: method.to_string(),
        params: parse_params(&params)?,
        id: Id::Num(42),
    }));
    Ok(request)
}

fn parse_params<T: Serialize>(params: &T) -> Result<Params> {
    let json = serde_json::to_value(params)?;

    match json {
        Value::Array(vec) => Ok(Params::Array(vec)),
        Value::Object(map) => Ok(Params::Map(map)),
        Value::Null => Ok(Params::None),
        _ => Err(anyhow!("parse params")),
    }
}

fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
    match response {
        Response::Single(output) => handle_output(output),
        _ => unreachable!(),
    }
}

fn handle_output<T: DeserializeOwned>(output: Output) -> Result<T> {
    let value = match output {
        Output::Success(succ) => succ.result,
        Output::Failure(_) => return Err(anyhow!("handle output")),
    };

    serde_json::from_value(value).map_err(anyhow::Error::new)
}
