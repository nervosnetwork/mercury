use crate::rpc_impl::{
    address_to_script, parse_normal_address, ACP_USED_CACHE, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY,
    INIT_ESTIMATE_FEE, MAX_ITEM_NUM, MIN_CKB_CAPACITY, STANDARD_SUDT_CAPACITY, TX_POOL_CACHE,
};

use crate::{error::RpcError, CkbRpc};

use common::utils::{
    decode_udt_amount, encode_udt_amount, parse_address, to_fixed_array, u128_sub, unwrap_only_one,
};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError};

use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{RationalU256, ScriptHashType, TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub, thread};
