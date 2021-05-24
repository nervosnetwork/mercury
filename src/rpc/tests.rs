use super::*;

use crate::extensions::tests::{build_extension, MemoryDB};
use crate::extensions::ExtensionType;
use crate::stores::BatchStore;

use ckb_indexer::{indexer::Indexer, store::Store};
use ckb_sdk::{Address, NetworkType};
use ckb_types::core::{
    capacity_bytes, BlockBuilder, Capacity, HeaderBuilder, ScriptHashType, TransactionBuilder,
};
use ckb_types::packed::{CellInput, CellOutputBuilder, Script, ScriptBuilder};
use ckb_types::{bytes::Bytes, prelude::*, H256};

use std::sync::Arc;

const SHANNON_PER_CKB: u64 = 100_000_000;

#[test]
fn test_rpc_get_ckb_balance() {
    let store = MemoryDB::new(0u32.to_string().as_str());
    let indexer = Arc::new(Indexer::new(store.clone(), 10, u64::MAX));
    let batch_store = BatchStore::create(store.clone()).unwrap();

    let ckb_ext = build_extension(
        &ExtensionType::CkbBalance,
        Default::default(),
        Arc::clone(&indexer),
        batch_store.clone(),
    );
    let rpc = MercuryRpcImpl::new(store, Default::default());

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

    assert_eq!(balance00.unwrap(), 1000 * SHANNON_PER_CKB);
    assert_eq!(balance01.unwrap(), 2000 * SHANNON_PER_CKB);
}
