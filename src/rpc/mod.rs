pub mod rpc_impl;

use ckb_types::bytes::Bytes;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

pub use rpc_impl::MercuryRpcImpl;

#[rpc(server)]
pub trait MercuryRpc {
    #[rpc(name = "get_ckb_balance")]
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>>;

    #[rpc(name = "get_sudt_balance")]
    fn get_sudt_balance(&self, sudt_id: Bytes, addr: String) -> RpcResult<Option<u128>>;
}
