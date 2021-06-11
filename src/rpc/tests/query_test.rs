use super::*;

fn query_test(
    rpc: &MercuryRpcImpl<MemoryDB>,
    addr: &str,
    expected_len: usize,
    ret_index: usize,
    expected_capacity: u64,
    expect_amount: Option<u128>,
) {
    let ret = rpc
        .get_cells_by_lock_script(&(parse_address(addr).unwrap().payload().into()))
        .unwrap();
    assert_eq!(ret.len(), expected_len);

    let capacity: u64 = ret[ret_index].0.cell_output.capacity().unpack();
    assert_eq!(capacity, expected_capacity * BYTE_SHANNONS);

    let data = ret[ret_index].0.cell_data.raw_data();
    if let Some(amount) = expect_amount {
        assert_eq!(amount.to_le_bytes().to_vec(), data);
    } else {
        assert!(data.is_empty());
    }
}

#[test]
fn test_get_cells_by_lock_script() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 0, 200, 0),
        AddressData::new(addr_3, 600_000, 0, 0),
    ]);

    let rpc = engine.rpc();
    let expected_len = 1usize;
    let ret_index = 0usize;

    query_test(&rpc, addr_1, expected_len, ret_index, 500_000, None);
    query_test(&rpc, addr_2, expected_len, ret_index, 142, Some(200));
    query_test(&rpc, addr_3, expected_len, ret_index, 600_000, None);
}

#[test]
fn test_get_ckb_balance() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 300, 0),
        AddressData::new(addr_2, 1000, 200, 100),
        // AddressData::new(addr_3, 600_000, 0, 0, 0),
    ]);

    let rpc = engine.rpc();
    let ret_1 = rpc.get_balance(None, addr_1.to_string()).unwrap();
    let ret_2 = rpc.get_balance(None, addr_2.to_string()).unwrap();

    assert_eq!(ret_1.owned, (500142 * BYTE_SHANNONS).to_string());
    assert_eq!(ret_2.owned, (1142 * BYTE_SHANNONS).to_string());
    assert_eq!(ret_2.locked, (142 * BYTE_SHANNONS).to_string());
}

#[test]
#[ignore]
fn test_get_ckb_balance_matured_cellbase() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let mut engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 100_000, 400, 100),
        AddressData::new(addr_2, 100_000, 0, 0),
    ]);

    let rpc = engine.rpc();
    let ret_1_at_genesis = rpc.get_balance(None, addr_1.to_string()).unwrap();
    let ret_2_at_genesis = rpc.get_balance(None, addr_2.to_string()).unwrap();
    assert_eq!(
        ret_1_at_genesis.owned,
        (100_000 * BYTE_SHANNONS).to_string()
    );
    // assert_eq!(
    //     ret_2_at_genesis.owned,
    //     (100_142 * BYTE_SHANNONS).to_string()
    // );
    assert_eq!(ret_1_at_genesis.locked, (142 * BYTE_SHANNONS).to_string());
    assert_eq!(ret_2_at_genesis.locked, (0).to_string());

    let cellbase_tx = RpcTestEngine::build_cellbase_tx(addr_1, 1000);
    let block_1 = RpcTestEngine::new_block(vec![cellbase_tx], 1, 1);
    engine.append(block_1);
    let ret_at_block_1 = rpc.get_balance(None, addr_1.to_string()).unwrap();
    assert_eq!(ret_at_block_1.locked, (100_142 * BYTE_SHANNONS).to_string());

    let cellbase_tx = RpcTestEngine::build_cellbase_tx(addr_1, 1000);
    let block_2 = RpcTestEngine::new_block(vec![cellbase_tx], 2, 10);
    engine.append(block_2);
    let ret_1_at_block_2 = rpc.get_balance(None, addr_1.to_string()).unwrap();
    assert_eq!(
        ret_1_at_genesis.owned,
        (200_000 * BYTE_SHANNONS).to_string()
    );
    assert_eq!(
        ret_1_at_block_2.locked,
        (100_142 * BYTE_SHANNONS).to_string()
    );
}

#[test]
fn test_get_udt_balance() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 300, 0),
        AddressData::new(addr_2, 0, 200, 100),
        // AddressData::new(addr_3, 600_000, 0, 0, 0),
    ]);

    let rpc = engine.rpc();
    let ret_1 = rpc
        .get_balance(Some(SUDT_HASH.read().clone()), addr_1.to_string())
        .unwrap();
    let ret_2 = rpc
        .get_balance(Some(SUDT_HASH.read().clone()), addr_2.to_string())
        .unwrap();

    assert_eq!(ret_1.owned, 300.to_string());
    assert_eq!(ret_2.owned, 300.to_string());
}
