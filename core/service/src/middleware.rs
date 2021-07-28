use core_rpc::CkbRpcClient;

use jsonrpc_core::futures_util::future::Either;
use jsonrpc_core::middleware::{Middleware, NoopCallFuture, NoopFuture};
use jsonrpc_core::{Call, Metadata, Request, Response};

use std::collections::HashSet;
use std::future::Future;

const SEND_ALERT: &str = "send_alert";
const GET_BLOCK: &str = "get_block";
const GET_BLOCK_BY_NUMBER: &str = "get_block_by_number";
const GET_HEADER: &str = "get_header";
const GET_HEADER_BY_NUMBER: &str = "get_header_by_number";
const GET_TRANSACTION: &str = "get_transaction";
const GET_BLOCK_HASH: &str = "get_block_hash";
const GET_TIP_HEADER: &str = "get_tip_header";
const GET_LIVE_CELL: &str = "get_live_cell";
const GET_TIP_BLOCK_NUMBER: &str = "get_tip_block_number";
const GET_CURRENT_EPOCH: &str = "get_current_epoch";
const GET_EPOCH_BY_NUMBER: &str = "get_epoch_by_number";
const GET_BLOCK_ECONOMIC_STATE: &str = "get_block_economic_state";
const GET_TRANSACTION_PROOF: &str = "get_transaction_proof";
const VERIFY_TRANSACTION_PROOF: &str = "verify_transaction_proof";
const GET_FORK_BLOCK: &str = "get_fork_block";
const GET_CONSENSUS: &str = "get_consensus";
const GET_BLOCK_MEDIAN_TIME: &str = "get_block_median_time";
const DRY_RUN_TRANSACTION: &str = "dry_run_transaction";
const CALCULATE_DAO_MAXIMUM_WITHDRAW: &str = "calculate_dao_maximum_withdraw";
const GET_BLOCK_TEMPLATE: &str = "get_block_template";
const SUBMIT_BLOCK: &str = "submit_block";
const LOCAL_NODE_INFO: &str = "local_node_info";
const GET_PEERS: &str = "get_peers";
const GET_BANNED_ADDRESSES: &str = "get_banned_addresses";
const CLEAR_BANNED_ADDRESSES: &str = "clear_banned_addresses";
const SET_BAN: &str = "set_ban";
const SYNC_STATE: &str = "sync_state";
const SET_NETWORK_ACTIVE: &str = "set_network_active";
const ADD_NODE: &str = "add_node";
const REMOVE_NODE: &str = "remove_node";
const PING_PEERS: &str = "ping_peers";
const SEND_TRANSACTION: &str = "send_transaction";
const TX_POOL_INFO: &str = "tx_pool_info";
const CLEAR_TX_POOL: &str = "clear_tx_pool";
const GET_RAW_TX_POOL: &str = "get_raw_tx_pool";
const GET_BLOCKCHAIN_INFO: &str = "get_blockchain_info";
const SUBSCRIBE: &str = "subscribe";
const UNSUBSCRIBE: &str = "unsubscribe";

lazy_static::lazy_static! {
    static ref CKB_RPC_REQ_SET: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert(SEND_ALERT.to_string());
        set.insert(GET_BLOCK.to_string());
        set.insert(GET_BLOCK_BY_NUMBER.to_string());
        set.insert(GET_HEADER.to_string());
        set.insert(GET_HEADER_BY_NUMBER.to_string());
        set.insert(GET_TRANSACTION.to_string());
        set.insert(GET_BLOCK_HASH.to_string());
        set.insert(GET_TIP_HEADER.to_string());
        set.insert(GET_LIVE_CELL.to_string());
        set.insert(GET_TIP_BLOCK_NUMBER.to_string());
        set.insert(GET_CURRENT_EPOCH.to_string());
        set.insert(GET_EPOCH_BY_NUMBER.to_string());
        set.insert(GET_BLOCK_ECONOMIC_STATE.to_string());
        set.insert(GET_TRANSACTION_PROOF.to_string());
        set.insert(VERIFY_TRANSACTION_PROOF.to_string());
        set.insert(GET_FORK_BLOCK.to_string());
        set.insert(GET_CONSENSUS.to_string());
        set.insert(GET_BLOCK_MEDIAN_TIME.to_string());
        set.insert(DRY_RUN_TRANSACTION.to_string());
        set.insert(CALCULATE_DAO_MAXIMUM_WITHDRAW.to_string());
        set.insert(GET_BLOCK_TEMPLATE.to_string());
        set.insert(SUBMIT_BLOCK.to_string());
        set.insert(LOCAL_NODE_INFO.to_string());
        set.insert(GET_PEERS.to_string());
        set.insert(GET_BANNED_ADDRESSES.to_string());
        set.insert(CLEAR_BANNED_ADDRESSES.to_string());
        set.insert(SET_BAN.to_string());
        set.insert(SYNC_STATE.to_string());
        set.insert(SET_NETWORK_ACTIVE.to_string());
        set.insert(ADD_NODE.to_string());
        set.insert(REMOVE_NODE.to_string());
        set.insert(PING_PEERS.to_string());
        set.insert(SEND_TRANSACTION.to_string());
        set.insert(TX_POOL_INFO.to_string());
        set.insert(CLEAR_TX_POOL.to_string());
        set.insert(GET_RAW_TX_POOL.to_string());
        set.insert(GET_BLOCKCHAIN_INFO.to_string());
        set.insert(SUBSCRIBE.to_string());
        set.insert(UNSUBSCRIBE.to_string());
        set
    };
}

#[derive(Default, Clone)]
pub struct RelayMetadata;

impl Metadata for RelayMetadata {}

pub struct CkbRelayMiddleware {
    ckb_client: CkbRpcClient,
}

impl<M: Metadata> Middleware<M> for CkbRelayMiddleware {
    type Future = NoopFuture;
    type CallFuture = NoopCallFuture;

    fn on_request<F, X>(&self, request: Request, meta: M, next: F) -> Either<Self::Future, X>
    where
        F: Fn(Request, M) -> X + Send + Sync,
        X: Future<Output = Option<Response>> + Send,
    {
        let req = match request.clone() {
            Request::Single(single) => single,
            Request::Batch(batch) => {
                if !batch.is_empty() {
                    batch.get(0).cloned().unwrap()
                } else {
                    return Either::Right(next(request, meta));
                }
            }
        };

        let method = match req {
            Call::MethodCall(inner) => inner,
            _ => return Either::Right(next(request, meta)),
        };

        if !CKB_RPC_REQ_SET.contains(&method.method) {
            return Either::Right(next(request, meta));
        }

        let id = method.id;
        let ckb_client = self.ckb_client.clone();

        Either::Left(Box::pin(
            async move { ckb_client.relay_exec(request, id).await },
        ))
    }
}

impl CkbRelayMiddleware {
    pub fn new(ckb_client: CkbRpcClient) -> Self {
        CkbRelayMiddleware { ckb_client }
    }
}
