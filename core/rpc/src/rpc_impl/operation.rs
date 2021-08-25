use crate::error::{InnerResult, RpcError, RpcErrorMessage};
use crate::rpc_impl::{address_to_script, minstant_elapsed, parse_normal_address};
use crate::types::Source;
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160};
use common::{Address, AddressPayload, MercuryError};
use core_storage::{DBAdapter, DBInfo, MercuryStore};

use ckb_jsonrpc_types::Status as TransactionStatus;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::HashMap;
use std::str::FromStr;

impl<C: CkbRpc + DBAdapter> MercuryRpcImpl<C> {
    pub(crate) async fn inner_register_addresses(
        &self,
        addresses: Vec<(H160, String)>,
    ) -> InnerResult<Vec<H160>> {
        let res = self.storage.register_addresses(addresses).await;
        let res = match res {
            Ok(res) => res,
            Err(error) => return Err(RpcErrorMessage::DBError(error.to_string())),
        };
        Ok(res)
    }
}
