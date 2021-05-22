use crate::extensions::{build_extensions, MATURE_THRESHOLD};
use crate::rpc::{MercuryRpc, MercuryRpcImpl};
use crate::{stores::BatchStore, types::ExtensionsConfig};

use ckb_indexer::indexer::Indexer;
use ckb_indexer::service::{gen_client, get_block_by_number, IndexerRpc, IndexerRpcImpl};
use ckb_indexer::store::{RocksdbStore, Store};
use ckb_sdk::NetworkType;
use ckb_types::core::{BlockNumber, BlockView, RationalU256};
use ckb_types::{packed, U256};
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{Server, ServerBuilder};
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use log::{error, info, trace};

use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::{sync::Arc, time::Duration};

const KEEP_NUM: u64 = 100;
const PRUNE_INTERVAL: u64 = 1000;
const GENESIS_NUMBER: u64 = 0;

// Adapted from https://github.com/nervosnetwork/ckb-indexer/blob/290ae55a2d2acfc3d466a69675a1a58fcade7f5d/src/service.rs#L25
// with extensions for more indexing features.
pub struct Service {
    store: RocksdbStore,
    poll_interval: Duration,
    listen_address: String,
    network_type: NetworkType,
    extensions_config: ExtensionsConfig,
    snapshot_interval: u64,
    snapshot_path: PathBuf,
    cellbase_maturity: RationalU256,
}

impl Service {
    pub fn new(
        store_path: &str,
        listen_address: &str,
        poll_interval: Duration,
        network_ty: &str,
        extensions_config: ExtensionsConfig,
        snapshot_interval: u64,
        snapshot_path: &str,
        cellbase_maturity: u64,
    ) -> Self {
        let store = RocksdbStore::new(store_path);
        let network_type = NetworkType::from_raw_str(network_ty).expect("invalid network type");
        let listen_address = listen_address.to_string();
        let snapshot_path = Path::new(snapshot_path).to_path_buf();
        let cellbase_maturity = RationalU256::from_u256(U256::from(cellbase_maturity));

        info!("Mercury running in CKB {:?}", network_type);

        Service {
            store,
            poll_interval,
            listen_address,
            network_type,
            extensions_config,
            snapshot_interval,
            snapshot_path,
            cellbase_maturity,
        }
    }

    pub fn start(&self) -> Server {
        let mut io_handler = IoHandler::new();
        let mercury_rpc_impl =
            MercuryRpcImpl::new(self.store.clone(), self.extensions_config.to_rpc_config());
        let indexer_rpc_impl = IndexerRpcImpl {
            store: self.store.clone(),
        };

        io_handler.extend_with(indexer_rpc_impl.to_delegate());
        io_handler.extend_with(mercury_rpc_impl.to_delegate());

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
                    .expect("listen_address parsed"),
            )
            .expect("Start Jsonrpc HTTP service")
    }

    #[allow(clippy::cmp_owned)]
    pub async fn poll(&self, rpc_client: gen_client::Client) {
        // 0.37.0 and above supports hex format
        let use_hex_format = loop {
            match rpc_client.local_node_info().await {
                Ok(local_node_info) => {
                    break local_node_info.version > "0.36".to_owned();
                }

                Err(err) => {
                    // < 0.32.0 compatibility
                    if format!("#{}", err).contains("missing field") {
                        break false;
                    }

                    error!("cannot get local_node_info from ckb node: {}", err);

                    std::thread::sleep(self.poll_interval);
                }
            }
        };

        self.run(rpc_client, use_hex_format).await;
    }

    async fn run(&self, rpc_client: gen_client::Client, use_hex_format: bool) {
        let mut tip = 0;

        loop {
            let batch_store =
                BatchStore::create(self.store.clone()).expect("batch store creation should be OK");
            let indexer = Arc::new(Indexer::new(batch_store.clone(), KEEP_NUM, u64::MAX));
            let extensions = build_extensions(
                self.network_type,
                &self.extensions_config,
                Arc::clone(&indexer),
                batch_store.clone(),
            )
            .expect("extension building failure");

            let append_block_func = |block: BlockView| {
                indexer.append(&block).expect("append block should be OK");
                extensions.iter().for_each(|extension| {
                    extension
                        .append(&block)
                        .unwrap_or_else(|e| panic!("append block error {:?}", e))
                });
            };

            // TODO: load tip first so extensions do not need to store their
            // own tip?
            let rollback_func = |tip_number: BlockNumber, tip_hash: packed::Byte32| {
                indexer.rollback().expect("rollback block should be OK");
                extensions.iter().for_each(|extension| {
                    extension
                        .rollback(tip_number, &tip_hash)
                        .unwrap_or_else(|e| panic!("rollback error {:?}", e))
                });
            };

            let mut prune = false;
            if let Some((tip_number, tip_hash)) = indexer.tip().expect("get tip should be OK") {
                tip = tip_number;

                match get_block_by_number(&rpc_client, tip_number + 1, use_hex_format).await {
                    Ok(Some(block)) => {
                        self.change_maturity_threshold(block.epoch().to_rational());

                        if block.parent_hash() == tip_hash {
                            info!("append {}, {}", block.number(), block.hash());
                            append_block_func(block.clone());
                            prune = (block.number() % PRUNE_INTERVAL) == 0;
                        } else {
                            info!("rollback {}, {}", tip_number, tip_hash);
                            rollback_func(tip_number, tip_hash);
                        }
                    }

                    Ok(None) => {
                        trace!("no new block");

                        std::thread::sleep(self.poll_interval);
                    }

                    Err(err) => {
                        error!("cannot get block from ckb node, error: {}", err);

                        std::thread::sleep(self.poll_interval);
                    }
                }
            } else {
                match get_block_by_number(&rpc_client, GENESIS_NUMBER, use_hex_format).await {
                    Ok(Some(block)) => {
                        self.change_maturity_threshold(block.epoch().to_rational());
                        append_block_func(block);
                    }

                    Ok(None) => {
                        error!("ckb node returns an empty genesis block");

                        std::thread::sleep(self.poll_interval);
                    }

                    Err(err) => {
                        error!("cannot get genesis block from ckb node, error: {}", err);

                        std::thread::sleep(self.poll_interval);
                    }
                }
            }

            batch_store.commit().expect("commit should be OK");

            if prune {
                let store = BatchStore::create(self.store.clone())
                    .expect("batch store creation should be OK");
                let indexer = Arc::new(Indexer::new(store.clone(), KEEP_NUM, PRUNE_INTERVAL));
                let extensions = build_extensions(
                    self.network_type,
                    &self.extensions_config,
                    Arc::clone(&indexer),
                    store.clone(),
                )
                .expect("extension building failure");

                if let Some((tip_number, tip_hash)) = indexer.tip().expect("get tip should be OK") {
                    indexer.prune().expect("indexer prune should be OK");

                    for extension in extensions.iter() {
                        extension
                            .prune(tip_number, &tip_hash, KEEP_NUM)
                            .expect("extension prune should be OK");
                    }
                }

                store.commit().expect("commit should be OK");
            }

            self.snapshot(tip);
        }
    }

    fn snapshot(&self, height: u64) {
        if height % self.snapshot_interval != 0 {
            return;
        }

        let mut path = self.snapshot_path.clone();
        path.push(height.to_string());
        let store = self.store.clone();

        tokio::spawn(async move {
            if let Err(e) = store.checkpoint(path) {
                error!("build {} checkpoint failed: {:?}", height, e);
            }
        });
    }

    fn change_maturity_threshold(&self, current_epoch: RationalU256) {
        if current_epoch < self.cellbase_maturity {
            return;
        }

        let new = current_epoch - self.cellbase_maturity.clone();
        let mut threshold = MATURE_THRESHOLD.write();
        *threshold = new;
    }
}
