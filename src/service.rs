use crate::{
    extensions::{build_extensions, BoxedExtension},
    types::ExtensionsConfig,
};
use anyhow::Result;
use ckb_indexer::{
    indexer::Indexer,
    service::{gen_client, get_block_by_number, IndexerRpc, IndexerRpcImpl},
    store::{RocksdbStore, Store},
};
use futures::future::Future;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{Server, ServerBuilder};
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use std::net::ToSocketAddrs;
use std::thread;
use std::time::Duration;

// Adapted from https://github.com/nervosnetwork/ckb-indexer/blob/290ae55a2d2acfc3d466a69675a1a58fcade7f5d/src/service.rs#L25
// with extensions for more indexing features.
pub struct Service {
    store: RocksdbStore,
    poll_interval: Duration,
    listen_address: String,
    extensions: Vec<BoxedExtension>,
}

impl Service {
    pub fn new(
        store_path: &str,
        listen_address: &str,
        poll_interval: Duration,
        extensions_config: ExtensionsConfig,
    ) -> Result<Self> {
        let store = RocksdbStore::new(store_path);
        let extensions = build_extensions(&extensions_config)?;
        Ok(Self {
            store,
            listen_address: listen_address.to_string(),
            poll_interval,
            extensions,
        })
    }

    pub fn start(&self) -> Server {
        let mut io_handler = IoHandler::new();
        let rpc_impl = IndexerRpcImpl {
            store: self.store.clone(),
        };
        io_handler.extend_with(rpc_impl.to_delegate());

        ServerBuilder::new(io_handler)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Null,
                AccessControlAllowOrigin::Any,
            ]))
            .health_api(("/ping", "ping"))
            .start_http(
                &self
                    .listen_address
                    .to_socket_addrs()
                    .expect("config listen_address parsed")
                    .next()
                    .expect("config listen_address parsed"),
            )
            .expect("Start Jsonrpc HTTP service")
    }

    pub fn poll(&self, rpc_client: gen_client::Client) {
        let indexer = Indexer::new(self.store.clone(), 100, 1000);
        // 0.37.0 and above supports hex format
        let use_hex_format = loop {
            match rpc_client.local_node_info().wait() {
                Ok(local_node_info) => {
                    break local_node_info.version > "0.36".to_owned();
                }
                Err(err) => {
                    // < 0.32.0 compatibility
                    if format!("#{}", err).contains("missing field") {
                        break false;
                    }
                    error!("cannot get local_node_info from ckb node: {}", err);
                    thread::sleep(self.poll_interval);
                }
            }
        };

        loop {
            if let Some((tip_number, tip_hash)) = indexer.tip().expect("get tip should be OK") {
                match get_block_by_number(&rpc_client, tip_number + 1, use_hex_format) {
                    Ok(Some(block)) => {
                        if block.parent_hash() == tip_hash {
                            info!("append {}, {}", block.number(), block.hash());
                            // TODO: all writes should be collapsed to a single transaction!
                            indexer.append(&block).expect("append block should be OK");
                            for extension in &self.extensions {
                                extension
                                    .append(&block)
                                    .expect("append block to extension should be OK");
                            }
                        } else {
                            info!("rollback {}, {}", tip_number, tip_hash);
                            // TODO: all writes should be collapsed to a single transaction!
                            // TODO: load tip first so extensions do not need to store their
                            // own tip?
                            indexer.rollback().expect("rollback block should be OK");
                            for extension in &self.extensions {
                                extension
                                    .rollback()
                                    .expect("rollback in extension should be OK");
                            }
                        }
                    }
                    Ok(None) => {
                        trace!("no new block");
                        thread::sleep(self.poll_interval);
                    }
                    Err(err) => {
                        error!("cannot get block from ckb node, error: {}", err);
                        thread::sleep(self.poll_interval);
                    }
                }
            } else {
                match get_block_by_number(&rpc_client, 0, use_hex_format) {
                    Ok(Some(block)) => indexer.append(&block).expect("append block should be OK"),
                    Ok(None) => {
                        error!("ckb node returns an empty genesis block");
                        thread::sleep(self.poll_interval);
                    }
                    Err(err) => {
                        error!("cannot get genesis block from ckb node, error: {}", err);
                        thread::sleep(self.poll_interval);
                    }
                }
            }
        }
    }
}
