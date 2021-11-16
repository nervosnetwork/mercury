use common::derive_more::Display;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum ClientError {
	#[display(fmt = "Ckb client error {}", _0)]
	ClientError(String),

	#[display(fmt = "Invalid rpc params {}", _0)]
    InvalidRpcParams(String),

	#[display(fmt = "Decode json error {}", _0)]
    DecodeJson(String),
}