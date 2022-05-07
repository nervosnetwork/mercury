use super::*;
use crate::r#impl::utils;

use ckb_jsonrpc_types::OutPoint;
use ckb_types::packed;
use common::lazy::SECP256K1_CODE_HASH;
use common::{Address, DetailedCell, NetworkType, Order, PaginationRequest, Range};
use core_rpc_types::lazy::{CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER};
use core_rpc_types::{
    indexer, AssetInfo, AssetType, Balance, DaoClaimPayload, DaoWithdrawPayload, ExtraType,
    From as From2, GetBalancePayload, GetBlockInfoPayload, Identity, IdentityFlag, Item, JsonItem,
    Mode, Record, SinceConfig, SinceFlag, SinceType, To, ToInfo, TransactionInfo,
};
use tokio::test;

async fn new_rpc(network: NetworkType) -> MercuryRpcImpl<CkbRpcClient> {
    let engine = RpcTestEngine::new_pg(network, "127.0.0.1").await;
    let rpc = engine.rpc(network);

    let tip = rpc.inner_get_tip(Context::new()).await.unwrap().unwrap();
    let tip_block_number = tip.block_number.into();
    let tip_epoch_number = rpc
        .get_epoch_by_number(Context::new(), tip_block_number)
        .await
        .unwrap();
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
        println!("tx_hash: {}", cell.out_point.tx_hash().to_string());
        println!("output_index: {}", cell.out_point.index());
        println!("cell_output: {}", cell.cell_output);
        let capacity: u64 = cell.cell_output.capacity().unpack();
        println!("capacity: {}", capacity);
        println!("cell_data: {}", hex::encode(cell.cell_data));
        println!(
            "address: {}",
            rpc.script_to_address(&cell.cell_output.lock()).to_string()
        );
    }
}

#[ignore]
#[test]
async fn test_get_live_cells_by_item() {
    let rpc = new_rpc(NetworkType::Dev).await;

    let out_point = new_outpoint(
        "0496b6d22aa0ac90592a79390d3c2d796a014879ae340682ff3774ad541f4228",
        0,
    );

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let mut page = PaginationRequest::default();
    let cells = rpc
        .get_live_cells_by_item(
            Context::new(),
            Item::OutPoint(out_point.into()),
            asset_infos,
            None,
            None,
            SECP256K1_CODE_HASH.get(),
            None,
            &mut page,
        )
        .await
        .unwrap();
    print_cells(&rpc, cells);
}
