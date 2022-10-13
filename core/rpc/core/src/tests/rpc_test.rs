use super::*;
use crate::r#impl::utils;

use ckb_jsonrpc_types::{JsonBytes, OutPoint, Script, ScriptHashType};
use ckb_types::packed::{self, Uint16, Uint64};
use common::lazy::SECP256K1_CODE_HASH;
use common::{Address, NetworkType, Order, PaginationRequest, Range};
use core_rpc_types::indexer::{ScriptType, SearchKey, SearchKeyFilter};
use core_rpc_types::lazy::{CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER};
use core_rpc_types::uints::JsonUint;
use core_rpc_types::{
    indexer, AssetInfo, AssetType, Balance, DaoClaimPayload, DaoWithdrawPayload, ExtraType,
    GetBalancePayload, GetBlockInfoPayload, Identity, IdentityFlag, Item, JsonItem, Record,
    SinceConfig, SinceFlag, SinceType, ToInfo, TransactionInfo,
};
use core_storage::DetailedCell;
use tokio::test;
use xsql_test::read_block_view;

const BLOCK_DIR: &str = "../../../devtools/test_data/blocks/";

async fn new_rpc(network: NetworkType, url: &str) -> MercuryRpcImpl<CkbRpcClient> {
    let engine = RpcTestEngine::new_pg(network, url).await;
    let rpc = engine.rpc(network);

    let tip = rpc.inner_get_tip().await.unwrap().unwrap();
    let tip_block_number = tip.block_number.into();
    let tip_epoch_number = rpc.get_epoch_by_number(tip_block_number).await.unwrap();
    CURRENT_BLOCK_NUMBER.swap(Arc::new(tip_block_number));
    CURRENT_EPOCH_NUMBER.swap(Arc::new(tip_epoch_number));

    rpc
}

fn new_outpoint(tx_id: &str, index: u32) -> packed::OutPoint {
    let tx_hash = H256::from_slice(&hex::decode(tx_id).unwrap())
        .unwrap()
        .pack();
    packed::OutPoint::new(tx_hash, index)
}

fn print_cells(rpc: &MercuryRpcImpl<CkbRpcClient>, cells: Vec<DetailedCell>) {
    println!("cells: {:?}", cells.len());
    for cell in cells {
        println!("*****************");
        println!("tx_hash: {}", cell.out_point.tx_hash());
        println!("output_index: {}", cell.out_point.index());
        println!("cell_output: {}", cell.cell_output);
        let capacity: u64 = cell.cell_output.capacity().unpack();
        println!("capacity: {}", capacity);
        println!("cell_data: {}", hex::encode(cell.cell_data));
        println!(
            "address: {}",
            rpc.script_to_address(&cell.cell_output.lock())
        );
    }
}

#[test]
async fn test_indexer_get_cells() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Dev);

    for i in 0..10 {
        engine
            .store
            .append_block(read_block_view(i, String::from(BLOCK_DIR).clone()).into())
            .await
            .unwrap();
    }

    let script = Script {
        code_hash: H256::default(),
        hash_type: ScriptHashType::Data,
        args: JsonBytes::from_vec(Vec::new()),
    };

    // with_data: Some(false)
    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: None,
        with_data: Some(false),
    };
    let cells = rpc
        .get_cells(search_key, Order::Asc, 7.into(), None)
        .await
        .unwrap()
        .objects;
    assert_eq!(7, cells.len());
    for (index, cell) in cells.iter().enumerate() {
        if vec![1, 2, 4].contains(&index) {
            assert!(cell.output.type_.is_some());
        }
        assert!(cell.output_data.is_none());
        assert_eq!(0, cell.block_number.value());
    }

    // with_data: None
    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: None,
        with_data: None,
    };
    let cells = rpc
        .get_cells(search_key, Order::Asc, 7.into(), None)
        .await
        .unwrap()
        .objects;
    assert_eq!(7, cells.len());
    for (index, cell) in cells.iter().enumerate() {
        if vec![1, 2, 4].contains(&index) {
            assert!(cell.output.type_.is_some());
        }
        assert!(cell.output_data.is_some());
        assert_eq!(0, cell.block_number.value());
    }

    // filter empty type script cells by setting script_len_range to [0, 1)
    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: Some(SearchKeyFilter {
            script: None,
            script_len_range: Some([0.into(), 1.into()]),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
    };
    let cells = rpc
        .get_cells(search_key, Order::Asc, 7.into(), None)
        .await
        .unwrap()
        .objects;
    assert_eq!(5, cells.len());
    for cell in cells {
        println!("{:?}", cell.out_point);
        assert!(cell.output.type_.is_none());
        assert!(cell.output_data.is_some());
        assert_eq!(0, cell.block_number.value());
    }

    // filter type script cells by setting script_len_range
    let search_key = SearchKey {
        script,
        script_type: ScriptType::Lock,
        filter: Some(SearchKeyFilter {
            script: None,
            script_len_range: Some([65.into(), 66.into()]),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
    };
    let cells = rpc
        .get_cells(search_key, Order::Asc, 7.into(), None)
        .await
        .unwrap()
        .objects;
    assert_eq!(3, cells.len());
    for cell in cells {
        assert!(cell.output.type_.is_some());
        assert!(cell.output_data.is_some());
        assert_eq!(0, cell.block_number.value());
    }

    // type script
    let script = Script {
        code_hash: h256!("0x00000000000000000000000000000000000000000000000000545950455f4944"),
        hash_type: ScriptHashType::Type,
        args: JsonBytes::from_vec(hex::decode("8536").unwrap()),
    };

    // filter type script cells by setting script_len_range
    let search_key = SearchKey {
        script,
        script_type: ScriptType::Type,
        filter: Some(SearchKeyFilter {
            script: None,
            script_len_range: Some([33.into(), 34.into()]),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
    };
    let cells = rpc
        .get_cells(search_key, Order::Asc, 7.into(), None)
        .await
        .unwrap()
        .objects;
    assert_eq!(1, cells.len());
    assert!(cells[0].output.type_.is_some());
    assert!(cells[0].output_data.is_some());
    assert_eq!(0, cells[0].block_number.value());
}

#[test]
async fn test_indexer_get_cells_capacity() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Dev);

    for i in 0..10 {
        engine
            .store
            .append_block(read_block_view(i, String::from(BLOCK_DIR).clone()).into())
            .await
            .unwrap();
    }

    let script = Script {
        code_hash: H256::default(),
        hash_type: ScriptHashType::Data,
        args: JsonBytes::from_vec(Vec::new()),
    };

    // filter empty type script cells by setting script_len_range to [0, 1)
    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: Some(SearchKeyFilter {
            script: None,
            script_len_range: Some([0.into(), 1.into()]),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
    };
    let cells_capacity = rpc.get_cells_capacity(search_key).await.unwrap();
    assert_eq!(cells_capacity.capacity, 840104890100000000.into());

    // filter type script cells by setting script_len_range
    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: Some(SearchKeyFilter {
            script: None,
            script_len_range: Some([65.into(), 66.into()]),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        }),
        with_data: Some(true),
    };
    let cells_capacity = rpc.get_cells_capacity(search_key).await.unwrap();
    assert_eq!(cells_capacity.capacity, 21600000000000.into());
}

#[test]
async fn test_indexer_get_transactions() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Dev);

    for i in 0..10 {
        engine
            .store
            .append_block(read_block_view(i, String::from(BLOCK_DIR).clone()).into())
            .await
            .unwrap();
    }

    let script = Script {
        code_hash: H256::default(),
        hash_type: ScriptHashType::Data,
        args: JsonBytes::from_vec(Vec::new()),
    };

    let search_key = SearchKey {
        script: script.clone(),
        script_type: ScriptType::Lock,
        filter: None,
        with_data: None,
    };
    let transactions = rpc
        .get_transactions(search_key, Order::Asc, 10.into(), None)
        .await
        .unwrap();
    let transactions = transactions.objects;
    for tx in transactions.iter() {
        println!("{:?}", tx);
    }
    assert_eq!(transactions.len(), 8);
}
