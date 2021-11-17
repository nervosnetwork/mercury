use super::*;
use tokio::test;

#[ignore]
#[tokio::test]
async fn test() {
    let engine = RpcTestEngine::new_pg(NetworkType::Mainnet, "127.0.0.1").await;
    let _rpc = engine.rpc(NetworkType::Mainnet);
}

#[ignore]
#[test]
async fn test_1() {
    init_debugger();

    let engine = RpcTestEngine::new_pg(NetworkType::Testnet, "47.242.31.83").await;
    let rpc = engine.rpc(NetworkType::Testnet);

    let mut asset_info = HashSet::new();
    asset_info.insert(crate::types::AssetInfo {
        asset_type: crate::types::AssetType::UDT,
        udt_hash: H256::from_str(
            "f21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
        )
        .unwrap(),
    });

    let payload = QueryTransactionsPayload {
        item: crate::types::JsonItem::Identity(String::from(
            "0x001a4ff63598e43af9cd42324abb7657fa849c5bc3",
        )),
        asset_infos: asset_info,
        structure_type: StructureType::Native,
        extra: None,
        block_range: None,
        pagination: common::PaginationRequest {
            cursor: Some(Bytes::from(vec![0, 0, 0, 0, 0, 0, 0, 12])),
            order: common::Order::Asc,
            limit: Some(1),
            skip: Some(0),
            return_count: false,
        },
    };

    rpc.query_transactions(payload).await.unwrap();
}
