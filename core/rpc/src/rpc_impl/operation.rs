use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{address_to_script, minstant_elapsed, parse_normal_address};
use crate::types::Source;
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{hash::blake2b_160, Address, AddressPayload, MercuryError, Result};
use core_storage::{DBInfo, RelationalStorage, Storage};

use ckb_jsonrpc_types::Status as TransactionStatus;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::HashMap;
use std::str::FromStr;

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
