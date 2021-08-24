mod operation_test;
// mod query_test;
// mod transfer_completion_test;

use crate::rpc_impl::{
    address_to_script, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, STANDARD_SUDT_CAPACITY,
};
use crate::types::{
    AdjustAccountPayload, AdvanceQueryPayload, BlockInfo, DepositPayload, GetBalancePayload,
    GetBalanceResponse, GetBlockInfoPayload, GetSpentTransactionPayload,
    GetTransactionInfoResponse, MercuryInfo, QueryResponse, QueryTransactionsPayload,
    SmartTransferPayload, TransactionCompletionResponse, TransactionStatus, TransferPayload,
    TxView, WithdrawPayload,
};
use crate::{CkbRpcClient, MercuryRpcImpl, MercuryRpcServer};

use common::utils::{decode_udt_amount, parse_address, ScriptInfo};
use common::{
    anyhow::Result, async_trait, hash::blake2b_160, Address, AddressPayload, NetworkType, ACP,
    CHEQUE, SECP256K1, SUDT,
};
use core_cli::config::{parse, MercuryConfig};
use core_storage::{DBAdapter, DBDriver, MercuryStore, XSQLPool};

use ckb_jsonrpc_types::Status as JsonTransactionStatus;
use ckb_types::core::{
    capacity_bytes, BlockBuilder, BlockView, Capacity, HeaderBuilder, RationalU256, ScriptHashType,
    TransactionBuilder, TransactionView,
};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use parking_lot::RwLock;
use rand::random;

use std::collections::{HashMap, HashSet};
use std::{str::FromStr, sync::Arc};

const CONFIG_PATH: &str = "../../devtools/config/testnet_config.toml";
const OUTPUT_FILE: &str = "../../free-space/output.json";
const NETWORK_TYPE: NetworkType = NetworkType::Testnet;
const MEMORY_DB: &str = ":memory:";

lazy_static::lazy_static! {
    pub static ref CELLBASE_ADDRESS: Address =
        Address::new(NetworkType::Testnet, packed::ScriptBuilder::default()
            .code_hash(H256(rand::random()).pack())
            .hash_type(ScriptHashType::Data.into())
            .args(Bytes::from(b"lock_script1".to_vec()).pack())
            .build()
            .into());
    pub static ref SUDT_HASH: RwLock<H256> = RwLock::new(Default::default());
}

#[macro_export]
macro_rules! hashset {
    () => {{
        HashSet::new()
    }};

    ($($input: expr), *) => {{
        let mut set = std::collections::HashSet::new();
        $(set.insert($input);)*
        set
    }};
}

fn init_debugger() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
}

pub struct RpcTestEngine {
    pub store: MercuryStore<CkbRpcClient>,
    pub script_map: HashMap<String, ScriptInfo>,
    pub config: MercuryConfig,
    pub sudt_script: packed::Script,
}

impl RpcTestEngine {
    pub async fn new() -> Self {
        let store = MercuryStore {
            inner: Arc::new(XSQLPool::new(
                CkbRpcClient::new(String::default()),
                100,
                0,
                0,
            )),
        };
        store
            .connect(DBDriver::SQLite, MEMORY_DB, "", 0, "", "")
            .await
            .unwrap();
        store.create_tables().await.unwrap();

        let config: MercuryConfig = parse(CONFIG_PATH).unwrap();
        let script_map = config.to_script_map();

        let sudt_script = script_map
            .get(SUDT)
            .cloned()
            .unwrap()
            .script
            .as_builder()
            .args(rand_bytes(32).pack())
            .build();

        let mut sudt_hash = SUDT_HASH.write();
        *sudt_hash = sudt_script.calc_script_hash().unpack();

        RpcTestEngine {
            store,
            script_map,
            config,
            sudt_script,
        }
    }

    pub async fn init_data(data: Vec<AddressData>) -> Self {
        let mut engine = RpcTestEngine::new().await;

        let cellbase = TransactionBuilder::default()
            .input(packed::CellInput::new_cellbase_input(0))
            .witness(packed::Script::default().into_witness())
            .output(
                packed::CellOutputBuilder::default()
                    .capacity(capacity_bytes!(1_000_000_000).pack())
                    .lock(CELLBASE_ADDRESS.clone().payload().into())
                    .build(),
            )
            .output_data(Default::default())
            .build();

        let mut block_builder = BlockBuilder::default().transaction(cellbase);

        for item in data.iter() {
            let addr = parse_address(&item.addr).unwrap();

            if item.ckb != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            packed::CellOutputBuilder::default()
                                .capacity(item.ckb.pack())
                                .lock(addr.payload().into())
                                .build(),
                        )
                        .output_data(Default::default())
                        .build(),
                );
            }

            if item.udt != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            packed::CellOutputBuilder::default()
                                .capacity(STANDARD_SUDT_CAPACITY.pack())
                                .type_(Some(engine.sudt_script.clone()).pack())
                                .lock(addr.payload().into())
                                .build(),
                        )
                        .output_data(item.udt.to_le_bytes().to_vec().pack())
                        .build(),
                );
            }

            if item.acp_udt != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            packed::CellOutputBuilder::default()
                                .capacity(STANDARD_SUDT_CAPACITY.pack())
                                .type_(Some(engine.sudt_script.clone()).pack())
                                .lock(
                                    engine
                                        .acp_builder()
                                        .args(addr.payload().args().pack())
                                        .build(),
                                )
                                .build(),
                        )
                        .output_data(item.acp_udt.to_le_bytes().to_vec().pack())
                        .build(),
                );
            }

            if item.cheque_udt != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            packed::CellOutputBuilder::default()
                                .capacity(CHEQUE_CELL_CAPACITY.pack())
                                .type_(Some(engine.sudt_script.clone()).pack())
                                .lock(
                                    engine
                                        .cheque_builder()
                                        .args(cheque_args(addr.payload()))
                                        .build(),
                                )
                                .build(),
                        )
                        .output_data(item.cheque_udt.to_le_bytes().to_vec().pack())
                        .build(),
                );
            }
        }

        let block = block_builder
            .header(HeaderBuilder::default().number(0.pack()).build())
            .build();

        engine.append(block).await;

        engine
    }

    pub fn build_cellbase_tx(miner_addr: &str, reward: u64) -> TransactionView {
        let addr = parse_address(&miner_addr).unwrap();
        TransactionBuilder::default()
            .witness(packed::Script::default().into_witness())
            .output(
                packed::CellOutputBuilder::default()
                    .capacity((reward * BYTE_SHANNONS).pack())
                    .lock(addr.payload().into())
                    .build(),
            )
            .output_data(Default::default())
            .build()
    }

    pub fn new_block(txs: Vec<TransactionView>, number: u64, epoch: u64) -> BlockView {
        let block_builder = BlockBuilder::default();
        let header = HeaderBuilder::default()
            .number(number.pack())
            .epoch(epoch.pack())
            .build();
        block_builder.transactions(txs).header(header).build()
    }

    pub async fn append(&mut self, block: BlockView) {
        self.store.append_block(block).await.unwrap();
    }

    pub fn rpc(&self) -> MercuryRpcImpl<CkbRpcClient> {
        MercuryRpcImpl::new(
            self.store.clone(),
            self.script_map.clone(),
            CkbRpcClient::new(String::new()),
            NetworkType::Testnet,
            RationalU256::from_u256(6u64.into()),
        )
    }

    fn acp_builder(&self) -> packed::ScriptBuilder {
        self.script_map
            .get(ACP)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    fn cheque_builder(&self) -> packed::ScriptBuilder {
        self.script_map
            .get(CHEQUE)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    pub fn get_db(&self) -> MercuryStore<CkbRpcClient> {
        self.store.clone()
    }
}

fn cheque_args(receiver: &AddressPayload) -> packed::Bytes {
    let sender = blake2b_160(
        address_to_script(
            parse_address("ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve")
                .unwrap()
                .payload(),
        )
        .as_slice(),
    );

    let mut ret = blake2b_160(address_to_script(receiver).as_slice()).to_vec();
    ret.extend_from_slice(&sender);
    ret.pack()
}

#[derive(Clone, Debug)]
pub struct AddressData {
    addr: String,
    ckb: u64,
    udt: u128,
    acp_udt: u128,
    cheque_udt: u128,
}

impl AddressData {
    fn new(addr: &str, ckb: u64, udt: u128, acp_udt: u128, cheque_udt: u128) -> AddressData {
        let addr = addr.to_string();
        let ckb = ckb * BYTE_SHANNONS;

        AddressData {
            addr,
            ckb,
            udt,
            acp_udt,
            cheque_udt,
        }
    }
}

pub fn rand_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
}

pub fn rand_h256() -> H256 {
    H256::from_slice(&rand_bytes(32)).unwrap()
}

fn write_file(data: String) {
    std::fs::write(OUTPUT_FILE, data).unwrap();
}
