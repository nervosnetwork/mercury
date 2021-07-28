#![allow(clippy::mutable_key_type)]

mod middleware;

use middleware::RelayMetadata;

use common::{anyhow::Result, NetworkType};
use core_extensions::{build_extensions, ExtensionsConfig, CURRENT_EPOCH, MATURE_THRESHOLD};
use core_rpc::{
    CkbRpc, CkbRpcClient, MercuryRpc, MercuryRpcImpl, CURRENT_BLOCK_NUMBER, TX_POOL_CACHE,
    USE_HEX_FORMAT,
};
use core_storage::{BatchStore, RocksdbStore, Store};

use ckb_indexer::indexer::Indexer;
use ckb_indexer::service::{IndexerRpc, IndexerRpcImpl};
use ckb_jsonrpc_types::RawTxPool;
use ckb_types::core::{BlockNumber, BlockView, RationalU256};
use ckb_types::{packed, H256, U256};
use jsonrpc_core::MetaIoHandler;
use jsonrpc_http_server::{Server, ServerBuilder};
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use log::{error, info, warn};
use rocksdb::{checkpoint::Checkpoint, DB};
use tokio::time::{sleep, Duration};

use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const KEEP_NUM: u64 = 100;
const PRUNE_INTERVAL: u64 = 1000;
const GENESIS_NUMBER: u64 = 0;

// Adapted from https://github.com/nervosnetwork/ckb-indexer/blob/290ae55a2d2acfc3d466a69675a1a58fcade7f5d/src/service.rs#L25
// with extensions for more indexing features.
pub struct Service {
    store: RocksdbStore,
    ckb_client: CkbRpcClient,
    poll_interval: Duration,
    listen_address: String,
    rpc_thread_num: usize,
    network_type: NetworkType,
    extensions_config: ExtensionsConfig,
    snapshot_interval: u64,
    snapshot_path: PathBuf,
    cellbase_maturity: RationalU256,
    cheque_since: U256,
}

impl Service {
    pub fn new(
        store_path: &str,
        listen_address: &str,
        poll_interval: Duration,
        rpc_thread_num: usize,
        network_ty: &str,
        extensions_config: ExtensionsConfig,
        snapshot_interval: u64,
        snapshot_path: &str,
        cellbase_maturity: u64,
        ckb_uri: String,
        cheque_since: u64,
    ) -> Self {
        let store = RocksdbStore::new(store_path);
        let ckb_client = CkbRpcClient::new(ckb_uri);
        let network_type = NetworkType::from_raw_str(network_ty).expect("invalid network type");
        let listen_address = listen_address.to_string();
        let snapshot_path = Path::new(snapshot_path).to_path_buf();
        let cellbase_maturity = RationalU256::from_u256(U256::from(cellbase_maturity));
        let cheque_since: U256 = cheque_since.into();

        info!("Mercury running in CKB {:?}", network_type);

        Service {
            store,
            ckb_client,
            poll_interval,
            listen_address,
            rpc_thread_num,
            network_type,
            extensions_config,
            snapshot_interval,
            snapshot_path,
            cellbase_maturity,
            cheque_since,
        }
    }

    pub fn init(&self) -> Server {
        let mut io_handler: MetaIoHandler<RelayMetadata, _> = MetaIoHandler::with_middleware(
            middleware::CkbRelayMiddleware::new(self.ckb_client.clone()),
        );
        let mercury_rpc_impl = MercuryRpcImpl::new(
            self.store.clone(),
            self.network_type,
            self.ckb_client.clone(),
            self.cheque_since.clone(),
            self.extensions_config.to_rpc_config(),
        );
        let indexer_rpc_impl = IndexerRpcImpl {
            version: "0.2.1".to_string(),
            store: self.store.clone(),
        };

        io_handler.extend_with(indexer_rpc_impl.to_delegate());
        io_handler.extend_with(mercury_rpc_impl.to_delegate());

        info!("Running!");

        ServerBuilder::new(io_handler)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Null,
                AccessControlAllowOrigin::Any,
            ]))
            .threads(self.rpc_thread_num)
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
    pub async fn start(&self) {
        // 0.37.0 and above supports hex format
        let use_hex_format = loop {
            match self.ckb_client.local_node_info().await {
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

        USE_HEX_FORMAT.swap(Arc::new(use_hex_format));
        let use_hex = use_hex_format;
        let client_clone = self.ckb_client.clone();

        tokio::spawn(async move {
            update_tx_pool_cache(client_clone, use_hex).await;
        });

        self.run(use_hex_format).await;
    }

    async fn run(&self, use_hex_format: bool) {
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
                extensions.iter().for_each(|extension| {
                    extension
                        .append(&block)
                        .unwrap_or_else(|e| panic!("append block error {:?}", e))
                });
                indexer.append(&block).expect("append block should be OK");
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

                match self
                    .get_block_by_number(tip_number + 1, use_hex_format)
                    .await
                {
                    Ok(Some(block)) => {
                        self.change_current_epoch(block.epoch().to_rational());

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
                        sleep(self.poll_interval).await;
                    }

                    Err(err) => {
                        error!("cannot get block from ckb node, error: {}", err);

                        sleep(self.poll_interval).await;
                    }
                }
            } else {
                match self
                    .get_block_by_number(GENESIS_NUMBER, use_hex_format)
                    .await
                {
                    Ok(Some(block)) => {
                        self.change_current_epoch(block.epoch().to_rational());
                        append_block_func(block);
                    }

                    Ok(None) => {
                        error!("ckb node returns an empty genesis block");

                        sleep(self.poll_interval).await;
                    }

                    Err(err) => {
                        error!("cannot get genesis block from ckb node, error: {}", err);

                        sleep(self.poll_interval).await;
                    }
                }
            }

            batch_store.commit().expect("commit should be OK");
            let _ = *CURRENT_BLOCK_NUMBER.swap(Arc::new(tip));

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

    async fn get_block_by_number(
        &self,
        block_number: BlockNumber,
        use_hex_format: bool,
    ) -> Result<Option<BlockView>> {
        self.ckb_client
            .get_block_by_number(block_number, use_hex_format)
            .await
            .map(|res| res.map(Into::into))
    }

    fn snapshot(&self, height: u64) {
        if height % self.snapshot_interval != 0 {
            return;
        }

        let mut path = self.snapshot_path.clone();
        path.push(height.to_string());
        let store = self.store.clone();

        tokio::spawn(async move {
            if let Err(e) = create_checkpoint(store.inner(), path) {
                error!("build {} checkpoint failed: {:?}", height, e);
            }
        });
    }

    fn change_current_epoch(&self, current_epoch: RationalU256) {
        self.change_maturity_threshold(current_epoch.clone());

        let mut epoch = CURRENT_EPOCH.write();
        *epoch = current_epoch;
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

fn create_checkpoint(db: &DB, path: PathBuf) -> Result<()> {
    Checkpoint::new(db)?.create_checkpoint(path)?;
    Ok(())
}

async fn update_tx_pool_cache(ckb_client: CkbRpcClient, use_hex_format: bool) {
    loop {
        match ckb_client.get_raw_tx_pool(Some(use_hex_format)).await {
            Ok(raw_pool) => handle_raw_tx_pool(&ckb_client, raw_pool).await,
            Err(e) => error!("get raw tx pool error {:?}", e),
        }

        sleep(Duration::from_millis(350)).await;
    }
}

async fn handle_raw_tx_pool(ckb_client: &CkbRpcClient, raw_pool: RawTxPool) {
    let mut input_set: HashSet<packed::OutPoint> = HashSet::new();
    let hashes = tx_hash_list(raw_pool);

    if let Ok(res) = ckb_client.get_transactions(hashes).await {
        for item in res.iter() {
            if let Some(tx) = item {
                for input in tx.transaction.inner.inputs.clone().into_iter() {
                    input_set.insert(input.previous_output.into());
                }
            } else {
                warn!("Get transaction from pool failed");
            }
        }
    }

    let mut pool_cache = TX_POOL_CACHE.write();
    *pool_cache = input_set;
}

fn tx_hash_list(raw_pool: RawTxPool) -> Vec<H256> {
    match raw_pool {
        RawTxPool::Ids(mut ids) => {
            let mut ret = ids.pending;
            ret.append(&mut ids.proposed);
            ret
        }
        RawTxPool::Verbose(map) => {
            let mut ret = map.pending.into_iter().map(|(k, _v)| k).collect::<Vec<_>>();
            let mut proposed = map
                .proposed
                .into_iter()
                .map(|(k, _v)| k)
                .collect::<Vec<_>>();

            ret.append(&mut proposed);
            ret
        }
    }
}
