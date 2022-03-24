use crate::utils::const_definition::{CKB_URI, MERCURY_URI, RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS};

use anyhow::{anyhow, Result};
use ckb_types::H256;
use jsonrpc_core::types::{
    Call, Id, MethodCall, Output, Params, Request, Response, Value, Version,
};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Serialize};

use std::thread::sleep;
use std::time::Duration;

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

    pub(crate) fn build_request<T: Serialize>(&self, method: String, params: T) -> Result<Request> {
        let request = Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method,
            params: parse_params(&params)?,
            id: Id::Num(42),
        }));
        Ok(request)
    }

    pub(crate) fn rpc_exec(&self, request: &Request) -> Result<Response> {
        let http_response = self.client.post(self.uri.as_str()).json(request).send()?;

        if !http_response.status().is_success() {
            return Err(anyhow!("http response"));
        }

        http_response.json().map_err(anyhow::Error::new)
    }
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

pub fn generate_block() -> Result<()> {
    let ckb_client = RpcClient::new(CKB_URI.to_string());
    let request = ckb_client
        .build_request("generate_block".to_string(), ())
        .expect("build request of generate_block");
    let response = ckb_client
        .rpc_exec(&request)
        .expect("call rpc generate_block");
    let block_hash: H256 = handle_response(response).expect("parse block hash");

    for _try in 0..=RPC_TRY_COUNT {
        let mercury_client = RpcClient::new(MERCURY_URI.to_string());
        let block_number: Option<u64> = None;
        let request = mercury_client
            .build_request(
                "get_block_info".to_string(),
                (block_number, block_hash.to_string()),
            )
            .expect("build request of get_block_info");
        let response = mercury_client.rpc_exec(&request);
        if response.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
    }

    return Err(anyhow!("generate block fail"));
}
