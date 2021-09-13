use super::*;

#[tokio::test]
async fn test() {
    let engine = RpcTestEngine::new_pg(NetworkType::Mainnet).await;
    let _rpc = engine.rpc(NetworkType::Mainnet);
}
