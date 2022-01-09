use crate::r#impl::MercuryRpcImpl;
use crate::InnerResult;

use common::Context;

use core_ckb_client::CkbRpc;
use core_rpc_types::axon::{InitChainPayload, SidechainConfig};
use core_rpc_types::TransactionCompletionResponse;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    fn inner_build_init_axon_chain_tx(
        &self,
        ctx: Context,
        payload: InitChainPayload,
    ) -> InnerResult<(TransactionCompletionResponse, SidechainConfig)> {
        todo!()
    }
}
