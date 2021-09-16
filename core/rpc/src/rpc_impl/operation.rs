use crate::error::{InnerResult, RpcErrorMessage};
use crate::{CkbRpc, MercuryRpcImpl};

use core_storage::Storage;

use ckb_types::H160;

impl<C: CkbRpc> MercuryRpcImpl<C> {
    pub(crate) async fn inner_register_addresses(
        &self,
        addresses: Vec<(H160, String)>,
    ) -> InnerResult<Vec<H160>> {
        self.storage
            .register_addresses(addresses)
            .await
            .map_err(|error| RpcErrorMessage::DBError(error.to_string()))
    }
}
