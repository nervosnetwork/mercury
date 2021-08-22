use crate::rpc_impl::{
    address_to_script, parse_normal_address, pubkey_to_secp_address, CURRENT_BLOCK_NUMBER,
};
use crate::types::{Balance, GetBalanceResponse};
use crate::{block_on, error::RpcError, CkbRpc};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError, Order};
use core_extensions::{
    ckb_balance, lock_time, script_hash, special_cells, udt_balance, DetailedCells, CKB_EXT_PREFIX,
    CURRENT_EPOCH, LOCK_TIME_PREFIX, SCRIPT_HASH_EXT_PREFIX, SP_CELL_EXT_PREFIX, UDT_EXT_PREFIX,
};
use core_storage::{add_prefix, IteratorDirection, Store};

use bincode::deserialize;
use ckb_indexer::indexer::{self, extract_raw_data, DetailedLiveCell, OutputIndex};
use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{packed, prelude::*, H160, H256};

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub};

use lazysort::SortedBy;
use num_traits::Zero;
