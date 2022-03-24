use crate::const_definition::{RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS};
use crate::mercury_types::SyncState;

use anyhow::{anyhow, Result};
use ckb_types::H256;
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

    pub fn generate_block(&self) -> Result<H256> {
        let request = build_request("generate_block".to_string(), ())
            .expect("build request of generate_block");
        let response = self
            .client
            .rpc_exec(&request)
            .expect("call rpc generate_block");
        handle_response(response)
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

    pub fn wait_block(&self, block_hash: H256) -> Result<()> {
        for _try in 0..=RPC_TRY_COUNT {
            let block_number: Option<u64> = None;
            let request = build_request(
                "get_block_info".to_string(),
                (block_number, block_hash.to_string()),
            )
            .expect("build request of get_block_info");
            let response = self.client.rpc_exec(&request);
            if response.is_ok() {
                return Ok(());
            }
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
        return Err(anyhow!("wait block fail"));
    }

    pub fn wait_sync(&self) {
        loop {
            let request = build_request("get_sync_state".to_string(), ()).expect("get sync state");
            let response = self.client.rpc_exec(&request).expect("exec rpc sync state");
            let sync_state: SyncState =
                handle_response(response).expect("handle response of sync state");
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

pub(crate) fn try_post_http_request(
    uri: &'static str,
    body: &'static str,
) -> Result<reqwest::blocking::Response> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(uri)
        .header("content-type", "application/json")
        .body(body)
        .send();
    resp.map_err(anyhow::Error::new)
}

pub(crate) fn post_http_request(uri: &'static str, body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(uri)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .unwrap();
    if !resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();

    serde_json::from_str(&text).unwrap()
}

#[derive(Clone, Debug)]
pub struct RpcClient {
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

pub(crate) fn build_request<T: Serialize>(method: String, params: T) -> Result<Request> {
    let request = Request::Single(Call::MethodCall(MethodCall {
        jsonrpc: Some(Version::V2),
        method,
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

pub(crate) fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
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
