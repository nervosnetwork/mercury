#![allow(unused_imports)]
#![allow(dead_code)]

mod block;
mod transaction;

pub use block::BlockTable;
pub use transaction::TransactionTable;

use crate::relational::{empty_rb_bytes, to_rb_bytes};

use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionView};
use ckb_types::{packed, prelude::*, H256};
use common::utils::to_fixed_array;
use common::DetailedCell;
use db_xsql::rbatis::{crud_table, Bytes as RbBytes};
use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::hash::{Hash, Hasher};

const BLAKE_160_HSAH_LEN: usize = 20;
const HASH256_LEN: usize = 32;
pub const IO_TYPE_INPUT: u8 = 0;
pub const IO_TYPE_OUTPUT: u8 = 1;
