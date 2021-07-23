mod memory_db;
mod operation_test;
mod query_test;
mod transfer_completion_test;

use memory_db::MemoryDB;

use crate::rpc_impl::{
    address_to_script, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, STANDARD_SUDT_CAPACITY,
};
use crate::types::{
    Action, CreateWalletPayload, FromAddresses, FromKeyAddresses, GetBalancePayload, QueryAddress,
    Source, ToAddress, ToKeyAddress, TransactionCompletionResponse, TransferItem, TransferPayload,
    WalletInfo,
};
use crate::{CkbRpcClient, MercuryRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, parse_address};
use common::{hash::blake2b_160, Address, AddressPayload, NetworkType};
use core_cli::config::{parse, MercuryConfig};
use core_extensions::{
    ckb_balance, lock_time, rce_validator, script_hash, special_cells, udt_balance, BoxedExtension,
    DeployedScriptConfig, Extension, ExtensionType, ExtensionsConfig, CKB_EXT_PREFIX,
    CURRENT_EPOCH, LOCK_TIME_PREFIX, MATURE_THRESHOLD, RCE_EXT_PREFIX, SCRIPT_HASH_EXT_PREFIX,
    SP_CELL_EXT_PREFIX, UDT_EXT_PREFIX,
};
use core_storage::{add_prefix, BatchStore, PrefixStore, Store};

use ckb_indexer::indexer::Indexer;
use ckb_jsonrpc_types::Status as TransactionStatus;
use ckb_types::core::{
    capacity_bytes, BlockBuilder, BlockView, Capacity, HeaderBuilder, RationalU256, ScriptHashType,
    TransactionBuilder, TransactionView,
};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256, U256};
use parking_lot::RwLock;
use rand::random;

use std::collections::{HashMap, HashSet};
use std::{str::FromStr, sync::Arc};

const CONFIG_PATH: &str = "../../devtools/config/testnet_config.toml";
const OUTPUT_FILE: &str = "../../free-space/output.json";
const NETWORK_TYPE: NetworkType = NetworkType::Testnet;

lazy_static::lazy_static! {
    pub static ref CELLBASE_ADDRESS: Address =
        Address::new(NetworkType::Testnet, packed::ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Data.into())
        .args(Bytes::from(b"lock_script1".to_vec()).pack())
        .build().into());
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

pub struct RpcTestEngine {
    pub store: MemoryDB,
    pub rpc_config: HashMap<String, DeployedScriptConfig>,
    pub json_configs: ExtensionsConfig,
    pub config: MercuryConfig,
    pub sudt_script: packed::Script,
}

impl RpcTestEngine {
    pub fn new() -> Self {
        let store = MemoryDB::create();
        let config: MercuryConfig = parse(CONFIG_PATH).unwrap();
        let json_configs: ExtensionsConfig = config.to_json_extensions_config().into();
        let rpc_config = json_configs.to_rpc_config();
        let sudt_script = rpc_config
            .get(udt_balance::SUDT)
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
            rpc_config,
            json_configs,
            config,
            sudt_script,
        }
    }

    fn batch_store(&self) -> BatchStore<MemoryDB> {
        BatchStore::create(self.store.clone()).unwrap()
    }

    fn indexer(&self, batch_store: BatchStore<MemoryDB>) -> Arc<Indexer<BatchStore<MemoryDB>>> {
        Arc::new(Indexer::new(batch_store, 10, u64::MAX))
    }

    fn build_extensions_list(
        &self,
        indexer: Arc<Indexer<BatchStore<MemoryDB>>>,
        batch_store: BatchStore<MemoryDB>,
    ) -> Vec<Box<dyn Extension>> {
        vec![
            build_extension(
                &ExtensionType::CkbBalance,
                self.json_configs
                    .enabled_extensions
                    .get(&ExtensionType::CkbBalance)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::UDTBalance,
                self.json_configs
                    .enabled_extensions
                    .get(&ExtensionType::UDTBalance)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::SpecialCells,
                self.json_configs
                    .enabled_extensions
                    .get(&ExtensionType::SpecialCells)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::Locktime,
                self.json_configs
                    .enabled_extensions
                    .get(&ExtensionType::Locktime)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::ScriptHash,
                self.json_configs
                    .enabled_extensions
                    .get(&ExtensionType::ScriptHash)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store,
            ),
        ]
    }

    pub fn init_data(data: Vec<AddressData>) -> Self {
        let mut engine = RpcTestEngine::new();

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

        engine.append(block);

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

    pub fn append(&mut self, block: BlockView) {
        let batch_store = self.batch_store();
        let indexer = self.indexer(batch_store.clone());
        indexer.append(&block).unwrap();

        self.chenge_current_epoch(block.epoch().to_rational());
        self.build_extensions_list(Arc::clone(&indexer), batch_store.clone())
            .iter()
            .for_each(|ext| ext.append(&block).unwrap());

        batch_store.commit().unwrap();
    }

    pub fn rpc(&self) -> MercuryRpcImpl<MemoryDB, CkbRpcClient> {
        MercuryRpcImpl::new(
            self.store.clone(),
            NetworkType::Testnet,
            CkbRpcClient::new(String::new()),
            6u64.into(),
            self.rpc_config.clone(),
        )
    }

    #[allow(dead_code)]
    pub fn display_db(&self) {
        self.store.display();
    }

    fn acp_builder(&self) -> packed::ScriptBuilder {
        self.rpc_config
            .get(special_cells::ACP)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    fn cheque_builder(&self) -> packed::ScriptBuilder {
        self.rpc_config
            .get(special_cells::CHEQUE)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    fn chenge_current_epoch(&self, current_epoch: RationalU256) {
        self.change_maturity_threshold(current_epoch.clone());

        let mut epoch = CURRENT_EPOCH.write();
        *epoch = current_epoch;
    }

    fn change_maturity_threshold(&self, current_epoch: RationalU256) {
        let cellbase_maturity = RationalU256::from_u256(U256::from(self.config.cellbase_maturity));
        if current_epoch < cellbase_maturity {
            return;
        }

        let new = current_epoch - cellbase_maturity;
        let mut threshold = MATURE_THRESHOLD.write();
        *threshold = new;
    }

    pub fn get_db(&self) -> MemoryDB {
        self.store.clone()
    }
}

pub fn build_extension<S: Store + 'static>(
    extension_type: &ExtensionType,
    script_config: HashMap<String, DeployedScriptConfig>,
    indexer: Arc<Indexer<S>>,
    batch_store: S,
) -> BoxedExtension {
    match extension_type {
        ExtensionType::RceValidator => Box::new(rce_validator::RceValidatorExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*RCE_EXT_PREFIX)),
            script_config,
        )),

        ExtensionType::CkbBalance => Box::new(ckb_balance::CkbBalanceExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*CKB_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),

        ExtensionType::UDTBalance => Box::new(udt_balance::UDTBalanceExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*UDT_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),

        ExtensionType::SpecialCells => Box::new(special_cells::SpecialCellsExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*SP_CELL_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),

        ExtensionType::Locktime => Box::new(lock_time::LocktimeExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*LOCK_TIME_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),

        ExtensionType::ScriptHash => Box::new(script_hash::ScriptHashExtension::new(
            PrefixStore::new_with_prefix(batch_store, Bytes::from(*SCRIPT_HASH_EXT_PREFIX)),
            Arc::clone(&indexer),
            NETWORK_TYPE,
            script_config,
        )),
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
