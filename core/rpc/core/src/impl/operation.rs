use crate::{error::CoreError, InnerResult, MercuryRpcImpl};

use common::Context;
use core_ckb_client::CkbRpc;
use core_storage::Storage;

use ckb_types::H160;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_register_addresses(
        &self,
        ctx: Context,
        addresses: Vec<(H160, String)>,
    ) -> InnerResult<Vec<H160>> {
        self.storage
            .register_addresses(ctx, addresses)
            .await
            .map_err(|error| CoreError::DBError(error.to_string()).into())
    }
}
