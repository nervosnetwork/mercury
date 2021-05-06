pub mod rpc_impl;

use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

pub use rpc_impl::MercuryRpcImpl;

#[rpc(server)]
pub trait MercuryRpc {
    #[rpc(name = "get_ckb_balance")]
    fn get_ckb_balance(&self, addr: String) -> RpcResult<Option<u64>>;

    #[rpc(name = "get_sudt_balance")]
    fn get_sudt_balance(&self, sudt_hash: H256, addr: String) -> RpcResult<Option<u128>>;

    #[rpc(name = "is_in_rce_list")]
    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool>;
}
