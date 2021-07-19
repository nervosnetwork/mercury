use super::*;

#[test]
fn test_ckb_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";

    let script_1 = address_to_script(parse_address(addr_1).unwrap().payload());
    let script_2 = address_to_script(parse_address(addr_2).unwrap().payload());
    let script_1_hash = blake2b_160(script_1.as_slice());
    let script_2_hash = blake2b_160(script_2.as_slice());

    let engine = RpcTestEngine::init_data(vec![AddressData::new(addr_1, 500_000, 0, 0, 0)]);

    let rpc = engine.rpc();

    let exist = engine
        .get_db()
        .exists(add_prefix(
            *SCRIPT_HASH_EXT_PREFIX,
            script_hash::Key::ScriptHash(script_1_hash).into_vec(),
        ))
        .unwrap();

    assert!(exist);

    let exist = engine
        .get_db()
        .exists(add_prefix(
            *SCRIPT_HASH_EXT_PREFIX,
            script_hash::Key::ScriptHash(script_2_hash).into_vec(),
        ))
        .unwrap();

    assert!(!exist);

    let hash = rpc.register_addresses(vec![addr_2.to_string()]).unwrap();
    assert_eq!(H160(script_2_hash), hash[0]);

    let exist = engine
        .get_db()
        .exists(add_prefix(
            *SCRIPT_HASH_EXT_PREFIX,
            script_hash::Key::ScriptHash(script_2_hash).into_vec(),
        ))
        .unwrap();

    assert!(exist);
}