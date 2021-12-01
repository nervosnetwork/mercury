use super::*;

#[ignore]
#[tokio::test]
async fn test() {
    let engine = RpcTestEngine::new_pg(NetworkType::Mainnet, "127.0.0.1").await;
    let _rpc = engine.rpc(NetworkType::Mainnet);
}
