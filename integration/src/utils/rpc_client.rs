use crate::const_definition::RPC_TRY_INTERVAL_SECS;

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::{EpochView, LocalNode, OutputsValidator, Transaction};
use ckb_types::H256;
use core_rpc_types::{
    AdjustAccountPayload, BlockInfo, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload,
    GetAccountInfoPayload, GetAccountInfoResponse, GetBalancePayload, GetBalanceResponse,
    GetBlockInfoPayload, GetTransactionInfoResponse, MercuryInfo, PaginationResponse,
    QueryTransactionsPayload, SimpleTransferPayload, SudtIssuePayload, SyncState,
    TransactionCompletionResponse, TransferPayload, TxView,
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

    pub fn get_sync_state(&self) -> Result<SyncState> {
        request(&self.client, "get_sync_state", ())
    }

    pub fn get_block_info(&self, block_hash: H256) -> Result<BlockInfo> {
        let payload = GetBlockInfoPayload {
            block_hash: Some(block_hash),
            block_number: None,
        };
        request(&self.client, "get_block_info", vec![payload])
    }

    pub fn query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> Result<PaginationResponse<TxView>> {
        request(&self.client, "query_transactions", vec![payload])
    }

    pub fn get_transaction_info(&self, tx_hash: H256) -> Result<GetTransactionInfoResponse> {
        request(&self.client, "get_transaction_info", vec![tx_hash])
    }

    pub fn get_account_info(
        &self,
        payload: GetAccountInfoPayload,
    ) -> Result<GetAccountInfoResponse> {
        request(&self.client, "get_account_info", vec![payload])
    }

    pub fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_transfer_transaction", vec![payload])
    }

    pub fn build_sudt_issue_transaction(
        &self,
        payload: SudtIssuePayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_sudt_issue_transaction", vec![payload])
    }

    pub fn build_adjust_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> Result<Option<TransactionCompletionResponse>> {
        request(
            &self.client,
            "build_adjust_account_transaction",
            vec![payload],
        )
    }

    pub fn build_simple_transfer_transaction(
        &self,
        payload: SimpleTransferPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(
            &self.client,
            "build_simple_transfer_transaction",
            vec![payload],
        )
    }

    pub fn build_dao_deposit_transaction(
        &self,
        payload: DaoDepositPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_dao_deposit_transaction", vec![payload])
    }

    pub fn build_dao_withdraw_transaction(
        &self,
        payload: DaoWithdrawPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(
            &self.client,
            "build_dao_withdraw_transaction",
            vec![payload],
        )
    }

    pub fn build_dao_claim_transaction(
        &self,
        payload: DaoClaimPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_dao_claim_transaction", vec![payload])
    }

    pub fn wait_block(&self, block_hash: H256) {
        while self.get_block_info(block_hash.clone()).is_err() {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }

    pub fn wait_sync(&self) {
        loop {
            let sync_state = if let Ok(sync_state) = self.get_sync_state() {
                sync_state
            } else {
                continue;
            };
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
        Output::Failure(_) => return Err(anyhow!("handle output: {:?}", output)),
    };

    serde_json::from_value(value).map_err(anyhow::Error::new)
}
