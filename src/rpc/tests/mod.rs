mod query_test;
mod transfer_completion;

use crate::config::{parse, MercuryConfig};
use crate::extensions::tests::{build_extension, MemoryDB};
use crate::extensions::{special_cells, udt_balance, Extension, ExtensionType};
use crate::rpc::rpc_impl::{BYTE_SHANNONS, STANDARD_SUDT_CAPACITY};
use crate::rpc::types::TransferCompletionResponse;
use crate::rpc::{MercuryRpc, MercuryRpcImpl};
use crate::stores::BatchStore;
use crate::types::{DeployedScriptConfig, ExtensionsConfig};
use crate::utils::{decode_udt_amount, parse_address};

use ckb_indexer::{indexer::Indexer, store::Store};
use ckb_sdk::{Address, NetworkType};
use ckb_types::core::{
    capacity_bytes, BlockBuilder, BlockView, Capacity, HeaderBuilder, ScriptHashType,
    TransactionBuilder,
};
use ckb_types::packed::{CellInput, CellOutputBuilder, Script, ScriptBuilder};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use parking_lot::RwLock;
use rand::random;

use std::collections::HashMap;
use std::sync::Arc;

const CONFIG_PATH: &str = "./devtools/config/config.toml";
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
    pub batch_store: BatchStore<MemoryDB>,
    pub extensions: Vec<Box<dyn Extension>>,
    pub config: HashMap<String, DeployedScriptConfig>,
    pub indexer: Arc<Indexer<BatchStore<MemoryDB>>>,
    pub sudt_script: packed::Script,
}

impl RpcTestEngine {
    pub fn new() -> Self {
        let store = MemoryDB::create();
        let batch_store = BatchStore::create(store.clone()).unwrap();
        let indexer = Arc::new(Indexer::new(batch_store.clone(), 10, u64::MAX));
        let config: MercuryConfig = parse(CONFIG_PATH).unwrap();
        let json_configs: ExtensionsConfig = config.to_json_extensions_config().into();
        let config = json_configs.to_rpc_config();

        let extensions = vec![
            build_extension(
                &ExtensionType::CkbBalance,
                json_configs
                    .enabled_extensions
                    .get(&ExtensionType::CkbBalance)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::UDTBalance,
                json_configs
                    .enabled_extensions
                    .get(&ExtensionType::UDTBalance)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
            build_extension(
                &ExtensionType::SpecialCells,
                json_configs
                    .enabled_extensions
                    .get(&ExtensionType::SpecialCells)
                    .cloned()
                    .unwrap(),
                Arc::clone(&indexer),
                batch_store.clone(),
            ),
        ];

        let sudt_script = config
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
            batch_store,
            extensions,
            config,
            indexer,
            sudt_script,
        }
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

            if item.acp_ckb != 0 {
                block_builder = block_builder.transaction(
                    TransactionBuilder::default()
                        .output(
                            CellOutputBuilder::default()
                                .capacity(item.acp_ckb.pack())
                                .lock(
                                    engine
                                        .acp_builder()
                                        .args(addr.payload().args().pack())
                                        .build(),
                                )
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

    pub fn append(&mut self, block: BlockView) {
        self.indexer.append(&block).unwrap();
        for ext in self.extensions.iter() {
            ext.append(&block).unwrap();
        }

        self.batch_store.clone().commit().unwrap();
    }

    pub fn rpc(&self) -> MercuryRpcImpl<MemoryDB> {
        MercuryRpcImpl::new(self.store.clone(), 6u64.into(), self.config.clone())
    }

    #[allow(dead_code)]
    pub fn display_db(&self) {
        self.store.display();
    }

    fn acp_builder(&self) -> packed::ScriptBuilder {
        self.config
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
    acp_ckb: u64,
    acp_sudt: u128,
}

impl AddressData {
    fn new(addr: &str, ckb: u64, sudt: u128, acp_ckb: u64, acp_sudt: u128) -> AddressData {
        let addr = addr.to_string();
        let ckb = ckb * BYTE_SHANNONS;
        let acp_ckb = acp_ckb * BYTE_SHANNONS;
        AddressData {
            addr,
            ckb,
            sudt,
            acp_ckb,
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

#[test]
fn test_rpc_get_ckb_balance() {
    let store = MemoryDB::new(0u32.to_string().as_str());
    let batch_store = BatchStore::create(store.clone()).unwrap();
    let indexer = Arc::new(Indexer::new(batch_store.clone(), 10, u64::MAX));

    let ckb_ext = build_extension(
        &ExtensionType::CkbBalance,
        Default::default(),
        Arc::clone(&indexer),
        batch_store.clone(),
    );
    let rpc = MercuryRpcImpl::new(store, 6u64.into(), Default::default());

    // setup test data
    let lock_script1 = ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Data.into())
        .args(Bytes::from(b"lock_script1".to_vec()).pack())
        .build();

    let lock_script2 = ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Type.into())
        .args(Bytes::from(b"lock_script2".to_vec()).pack())
        .build();

    let type_script1 = ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Data.into())
        .args(Bytes::from(b"type_script1".to_vec()).pack())
        .build();

    let type_script2 = ScriptBuilder::default()
        .code_hash(H256(rand::random()).pack())
        .hash_type(ScriptHashType::Type.into())
        .args(Bytes::from(b"type_script2".to_vec()).pack())
        .build();

    let cellbase0 = TransactionBuilder::default()
        .input(CellInput::new_cellbase_input(0))
        .witness(Script::default().into_witness())
        .output(
            CellOutputBuilder::default()
                .capacity(capacity_bytes!(1000).pack())
                .lock(lock_script1.clone())
                .build(),
        )
        .output_data(Default::default())
        .build();

    let tx00 = TransactionBuilder::default()
        .output(
            CellOutputBuilder::default()
                .capacity(capacity_bytes!(1000).pack())
                .lock(lock_script1.clone())
                .type_(Some(type_script1).pack())
                .build(),
        )
        .output_data(Default::default())
        .build();

    let tx01 = TransactionBuilder::default()
        .output(
            CellOutputBuilder::default()
                .capacity(capacity_bytes!(2000).pack())
                .lock(lock_script2.clone())
                .type_(Some(type_script2).pack())
                .build(),
        )
        .output_data(Default::default())
        .build();

    let block0 = BlockBuilder::default()
        .transaction(cellbase0)
        .transaction(tx00)
        .transaction(tx01)
        .header(HeaderBuilder::default().number(0.pack()).build())
        .build();

    ckb_ext.append(&block0).unwrap();
    batch_store.commit().unwrap();

    let block_hash = block0.hash();
    let unpack_0: H256 = block_hash.unpack();
    let unpack_1: [u8; 32] = block_hash.unpack();
    assert_eq!(unpack_0.as_bytes(), unpack_1.as_ref());

    let addr00 = Address::new(NetworkType::Testnet, lock_script1.into());
    let addr01 = Address::new(NetworkType::Testnet, lock_script2.into());
    let balance00 = rpc.get_ckb_balance(addr00.to_string()).unwrap();
    let balance01 = rpc.get_ckb_balance(addr01.to_string()).unwrap();

    assert_eq!(balance00.unwrap(), 1000 * BYTE_SHANNONS);
    assert_eq!(balance01.unwrap(), 2000 * BYTE_SHANNONS);
}
