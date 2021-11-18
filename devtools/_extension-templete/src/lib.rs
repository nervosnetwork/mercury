#[cfg(test)]
mod tests;
pub mod types;

use common::{async_trait, utils::ScriptInfo, Context, PaginationResponse, Result};
use core_rpc_types::TransactionCompletionResponse;
use core_rpc_utility::RpcUtility;
use core_storage::ExtensionStorage;
use protocol::extension::Extension;

use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::core::{BlockNumber, BlockView};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use jsonrpsee_http_server::types::Error;
use jsonrpsee_proc_macros::rpc;

use std::collections::HashMap;

#[rpc(server)]
pub trait ExtensionRpc {
    #[method(name = "extension")]
    async fn extension(&self, _payload: u32) -> Result<u32, Error>;
}

pub struct TempleteExtension<SE, RU> {
    _store: SE,
    _utils: RU,
    _builtin_scripts: HashMap<String, ScriptInfo>,
    _extra_config: Bytes,
}

// Implement the Extension trait here.
impl<SE, RU> Extension for TempleteExtension<SE, RU>
where
    SE: ExtensionStorage + Sync + Send + 'static,
    RU: RpcUtility + Send + Sync + 'static,
{
    fn append(&self, _block: &BlockView) -> Result<()> {
        Ok(())
    }

    fn rollback(&self, _tip_number: BlockNumber, _tip_hash: &H256) -> Result<()> {
        Ok(())
    }

    fn prune(&self, _tip_number: BlockNumber, _tip_hash: &H256, _keep_num: u64) -> Result<()> {
        Ok(())
    }
}

// Implement RPC server trait here.
#[async_trait]
impl<SE, RU> ExtensionRpcServer for TempleteExtension<SE, RU>
where
    SE: ExtensionStorage + Sync + Send + 'static,
    RU: RpcUtility + Send + Sync + 'static,
{
	async fn extension(&self, _payload: u32) -> Result<u32, Error> {
		Ok(0)
	}
}

impl<SE, RU> TempleteExtension<SE, RU>
where
    SE: ExtensionStorage + Sync + Send + 'static,
    RU: RpcUtility + Send + Sync + 'static,
{
    pub fn new(
        _store: SE,
        _utils: RU,
        _builtin_scripts: HashMap<String, ScriptInfo>,
        _extra_config: Bytes,
    ) -> Self {
        TempleteExtension {
            _store,
            _utils,
            _builtin_scripts,
            _extra_config,
        }
    }
}
