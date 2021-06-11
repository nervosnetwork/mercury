mod query_test;
mod transfer_completion;

use crate::config::{parse, MercuryConfig};
use crate::extensions::tests::{build_extension, MemoryDB};
use crate::extensions::{special_cells, udt_balance, Extension, ExtensionType};
use crate::rpc::rpc_impl::{BYTE_SHANNONS, STANDARD_SUDT_CAPACITY};
use crate::rpc::types::{
    Action, CreateWalletPayload, FromAccount, Source, ToAccount, TransactionCompletionResponse,
    TransferItem, TransferPayload, WalletInfo,
};
use crate::rpc::{MercuryRpc, MercuryRpcImpl};
use crate::stores::BatchStore;
use crate::types::{DeployedScriptConfig, ExtensionsConfig};
use crate::utils::{decode_udt_amount, parse_address};

use ckb_indexer::indexer::Indexer;
use ckb_sdk::{Address, NetworkType};
use ckb_types::core::{
    capacity_bytes, BlockBuilder, BlockView, Capacity, HeaderBuilder, ScriptHashType,
    TransactionBuilder, TransactionView,
};
use ckb_types::packed::{CellInput, CellOutputBuilder, Script, ScriptBuilder};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use parking_lot::RwLock;
use rand::random;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const CONFIG_PATH: &str = "./devtools/config/testnet_config.toml";
const OUTPUT_FILE: &str = "./free-space/output.json";

lazy_static::lazy_static! {
    pub static ref CELLBASE_ADDRESS: Address =
        Address::new(NetworkType::Testnet, ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Data.into())
        .args(Bytes::from(b"lock_script1".to_vec()).pack())
        .build().into());
    pub static ref SUDT_HASH: RwLock<H256> = RwLock::new(Default::default());
}

// macro_rules! transaction {
//     ([$($input: expr), *], [$($output: expr), *]) => {
//         let (mut inputs, mut outputs, mut data) = (vec![], vec![], vec![]);
//         $(inputs.push(
//             packed::CellInputBuilder::default()
//                 .previous_output(input)
//                 .build()
//             );
//         )*

//         $(
//             outputs.push($output.cell_output);
//             data.push($output.cell_data);
//         )*

//         TransactionBuilder::default()
//             .witness(Script::default().into_witness())
//             .inputs(inputs).outputs(outputs)
//             .outputs_data(data)
//             .build()
//     };
// }

pub struct RpcTestEngine {
    pub store: MemoryDB,
    pub rpc_config: HashMap<String, DeployedScriptConfig>,
    pub json_configs: ExtensionsConfig,
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
                batch_store,
            ),
        ]
    }

    pub fn init_data(data: Vec<AddressData>) -> Self {
        let mut engine = RpcTestEngine::new();

        let cellbase = TransactionBuilder::default()
            .input(CellInput::new_cellbase_input(0))
            .witness(Script::default().into_witness())
            .output(
                CellOutputBuilder::default()
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
                            CellOutputBuilder::default()
                                .capacity(item.ckb.pack())
                                .lock(addr.payload().into())
                                .build(),
                        )
                        .output_data(Default::default())
                        .build(),
                );
            }

            if item.sudt != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            CellOutputBuilder::default()
                                .capacity(STANDARD_SUDT_CAPACITY.pack())
                                .type_(Some(engine.sudt_script.clone()).pack())
                                .lock(addr.payload().into())
                                .build(),
                        )
                        .output_data(item.sudt.to_le_bytes().to_vec().pack())
                        .build(),
                );
            }

            if item.acp_sudt != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            CellOutputBuilder::default()
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
                        .output_data(item.acp_sudt.to_le_bytes().to_vec().pack())
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
            .witness(Script::default().into_witness())
            .output(
                CellOutputBuilder::default()
                    .capacity(reward.pack())
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

        self.build_extensions_list(Arc::clone(&indexer), batch_store.clone())
            .iter()
            .for_each(|ext| ext.append(&block).unwrap());

        batch_store.commit().unwrap();
    }

    pub fn rpc(&self) -> MercuryRpcImpl<MemoryDB> {
        MercuryRpcImpl::new(
            self.store.clone(),
            NetworkType::Testnet,
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
}

#[derive(Clone, Debug)]
pub struct AddressData {
    addr: String,
    ckb: u64,
    sudt: u128,
    acp_sudt: u128,
}

impl AddressData {
    fn new(addr: &str, ckb: u64, sudt: u128, acp_sudt: u128) -> AddressData {
        let addr = addr.to_string();
        let ckb = ckb * BYTE_SHANNONS;

        AddressData {
            addr,
            ckb,
            sudt,
            acp_sudt,
        }
    }
}

fn rand_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|_| random::<u8>()).collect::<Vec<_>>()
}

fn write_file(data: String) {
    std::fs::write(OUTPUT_FILE, data).unwrap();
}
