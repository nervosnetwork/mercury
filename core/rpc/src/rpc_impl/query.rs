use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, CURRENT_BLOCK_NUMBER,
};
use crate::types::{Balance, GetBalanceResponse};
use crate::{error::RpcError, CkbRpc};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError, Order};

use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{packed, prelude::*, H160, H256};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

use lazysort::SortedBy;
use num_traits::Zero;
