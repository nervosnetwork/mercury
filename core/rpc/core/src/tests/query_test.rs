use super::*;

use core_rpc_types::StructureType;

use tokio::test;

#[test]
async fn test_get_db_info() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Testnet);
    let db_info = rpc.get_db_info().unwrap();
    println!("db info: {:?}", db_info);
    assert_eq!(db_info.db, DBDriver::PostgreSQL);
    assert_eq!(db_info.center_id, 0);
    assert_eq!(db_info.machine_id, 0);
    assert_eq!(db_info.conn_size, 100);
}

#[test]
async fn test_get_spent_transaction() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Testnet);

    let outpoint = ckb_jsonrpc_types::OutPoint {
        tx_hash: h256!("0xb50ef2272f9f72b11e21ec12bd1b8fc9136cafc25c197b6fd4c2eb4b19fa905c"),
        index: 0u32.into(),
    };
    let payload = GetSpentTransactionPayload {
        outpoint,
        structure_type: StructureType::Native,
    };
    let res = rpc.get_spent_transaction(payload).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("10090"))
}
