#![allow(clippy::mutable_key_type, dead_code)]

mod middleware;

// use middleware::{CkbRelayMiddleware, RelayMetadata};

use common::{anyhow::Result, NetworkType};
use core_rpc::{
    types::ScriptInfo, CkbRpc, CkbRpcClient, MercuryRpcImpl, MercuryRpcServer,
    CURRENT_BLOCK_NUMBER, TX_POOL_CACHE, USE_HEX_FORMAT,
};
use core_storage::{DBDriver, MercuryStore};

use ckb_jsonrpc_types::RawTxPool;
use ckb_types::core::{BlockNumber, BlockView, RationalU256};
use ckb_types::{packed, H256, U256};
use jsonrpsee_http_server::HttpServerBuilder;
use log::{error, info, warn};
use parking_lot::RwLock;
use tokio::time::{sleep, Duration};

use std::collections::{HashMap, HashSet};
use std::net::ToSocketAddrs;
use std::sync::Arc;

const GENESIS_NUMBER: u64 = 0;

lazy_static::lazy_static! {
    pub static ref CURRENT_EPOCH: RwLock<RationalU256> = RwLock::new(RationalU256::one());
}

pub struct Service {
    store: MercuryStore<CkbRpcClient>,
    ckb_client: CkbRpcClient,
    poll_interval: Duration,
    listen_address: String,
    rpc_thread_num: usize,
    network_type: NetworkType,
    builtin_scripts: HashMap<String, ScriptInfo>,
    flush_cache_interval: u64,
    cellbase_maturity: RationalU256,
    cheque_since: U256,
}

impl Service {
    pub fn new(
        max_connections: u32,
        center_id: u16,
        machine_id: u16,
        listen_address: &str,
        poll_interval: Duration,
        rpc_thread_num: usize,
        network_ty: &str,
        builtin_scripts: HashMap<String, ScriptInfo>,
        flush_cache_interval: u64,
        cellbase_maturity: u64,
        ckb_uri: String,
        cheque_since: u64,
    ) -> Self {
        let ckb_client = CkbRpcClient::new(ckb_uri);
        let store = MercuryStore::new(ckb_client.clone(), max_connections, center_id, machine_id);
        let network_type = NetworkType::from_raw_str(network_ty).expect("invalid network type");
        let listen_address = listen_address.to_string();
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
            builtin_scripts,
            flush_cache_interval,
            cellbase_maturity,
            cheque_since,
        }
    }

    pub async fn init(
        &self,
        db_driver: String,
        db_name: String,
        host: String,
        port: u16,
        user: String,
        password: String,
    ) {
        self.store
            .connect(
                DBDriver::from_str(&db_driver),
                &db_name,
                &host,
                port,
                &user,
                &password,
            )
            .await
            .unwrap();

        let server = HttpServerBuilder::default()
            .build(
                self.listen_address
                    .to_socket_addrs()
                    .expect("config listen_address parsed")
                    .next()
                    .expect("listen_address parsed"),
            )
            .unwrap();

        // let mut io_handler: MetaIoHandler<RelayMetadata, _> =
        //     MetaIoHandler::with_middleware(CkbRelayMiddleware::new(self.ckb_client.clone()));
        let mercury_rpc_impl =
            MercuryRpcImpl::new(self.store.clone(), self.builtin_scripts.clone());

        info!("Running!");

        server.start(mercury_rpc_impl.into_rpc()).await.unwrap();
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
        let interval = self.flush_cache_interval;

        tokio::spawn(async move {
            update_tx_pool_cache(client_clone, interval, use_hex).await;
        });

        self.run(use_hex_format).await;
    }

    async fn run(&self, use_hex_format: bool) {
        let mut tip = 0;

        loop {
            if let Some((tip_number, _tip_hash)) =
                self.store.get_tip().await.expect("get tip should be OK")
            {
                tip = tip_number;

                match self
                    .get_block_by_number(tip_number + 1, use_hex_format)
                    .await
                {
                    Ok(Some(block)) => {
                        self.change_current_epoch(block.epoch().to_rational());
                        self.store.append_block(block).await.unwrap();
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
                        self.store.append_block(block).await.unwrap();
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

            let _ = *CURRENT_BLOCK_NUMBER.swap(Arc::new(tip));
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

    fn change_current_epoch(&self, current_epoch: RationalU256) {
        let mut epoch = CURRENT_EPOCH.write();
        *epoch = current_epoch;
    }
}

async fn update_tx_pool_cache(
    ckb_client: CkbRpcClient,
    flush_cache_interval: u64,
    use_hex_format: bool,
) {
    loop {
        match ckb_client.get_raw_tx_pool(Some(use_hex_format)).await {
            Ok(raw_pool) => handle_raw_tx_pool(&ckb_client, raw_pool).await,
            Err(e) => error!("get raw tx pool error {:?}", e),
        }

        sleep(Duration::from_millis(flush_cache_interval)).await;
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
