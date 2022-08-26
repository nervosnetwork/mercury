use super::*;

use tokio::test;

#[test]
async fn test_register_addresses() {
    let engine = RpcTestEngine::new().await;
    let rpc = engine.rpc(NetworkType::Testnet);

    init_debugger();
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";

    let script_1 = address_to_script(parse_address(addr_1).unwrap().payload());
    let script_2 = address_to_script(parse_address(addr_2).unwrap().payload());
    let script_1_hash = blake2b_160(script_1.as_slice());
    let script_2_hash = blake2b_160(script_2.as_slice());

    let hashes: Vec<H160> = rpc
        .register_addresses(vec![addr_1.to_string(), addr_2.to_string()])
        .await
        .unwrap();
    assert_eq!(H160(script_1_hash), hashes[0]);
    assert_eq!(H160(script_2_hash), hashes[1]);

    let address = engine
        .get_db()
        .get_registered_address(H160(script_1_hash))
        .await
        .unwrap();
    assert_eq!(Some(addr_1.to_owned()), address);
}

#[test]
async fn test_address() {
    let engine = RpcTestEngine::new().await;
    let _ = engine.rpc(NetworkType::Testnet);

    let addr = Address::from_str("ckt1qyp07nuu3fpu9rksy677uvchlmyv9ce5saes824qjq").unwrap();
    let script = address_to_script(addr.payload());
    assert_eq!(
        hex::encode(script.code_hash().raw_data()),
        "3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356".to_string()
    );
}
