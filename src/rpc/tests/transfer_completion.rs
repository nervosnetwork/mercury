use super::*;

fn response_assert(
    response: &TransactionCompletionResponse,
    expected_input_len: usize,
    expected_output_len: usize,
    expected_sigs_len: usize,
) {
    let cell_deps = response.tx_view.inner.cell_deps.clone();
    let cell_deps_len = cell_deps.len();
    let tmp_set = cell_deps.into_iter().collect::<HashSet<_>>();

    assert_eq!(cell_deps_len, tmp_set.len());
    assert_eq!(response.tx_view.inner.inputs.len(), expected_input_len);
    assert_eq!(response.tx_view.inner.outputs.len(), expected_output_len);
    assert_eq!(response.sigs_entry.len(), expected_sigs_len);
}

// ********************************
// Ckb transfer completion tests
// ********************************
#[test]
fn test_ckb_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 0, 200, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_1.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_2.to_string(),
                action: Action::PayByFrom,
            },
            amount: 100u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 1, 2, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_1.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (100 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[1].capacity,
        ((500_000 - 100 - 100) * BYTE_SHANNONS).into()
    );
}

#[test]
fn test_ckb_transfer_to_accounts_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 0, 200, 0),
        AddressData::new(addr_3, 80, 0, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_1.to_string()],
            source: Source::Owned,
        },
        items: vec![
            TransferItem {
                to: ToAccount {
                    ident: addr_2.to_string(),
                    action: Action::PayByFrom,
                },
                amount: 100u128,
            },
            TransferItem {
                to: ToAccount {
                    ident: addr_3.to_string(),
                    action: Action::PayByFrom,
                },
                amount: 100u128,
            },
        ],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 1, 3, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_1.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (100 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (100 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[2].capacity,
        ((500_000 - 100 - 100 - 100) * BYTE_SHANNONS).into()
    );
}

#[test]
fn test_list_ckb_cell_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500, 0, 0),
        AddressData::new(addr_2, 0, 200, 0),
        AddressData::new(addr_3, 500_000, 0, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_1.to_string(), addr_3.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_2.to_string(),
                action: Action::PayByFrom,
            },
            amount: 800u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 2, 2, 2);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_1.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 1);
    assert_eq!(ret.sigs_entry[1].pub_key, addr_3.to_string());
    assert_eq!(ret.sigs_entry[1].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (800 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[1].capacity,
        ((500_000 + 500 - 800 - 100) * BYTE_SHANNONS).into()
    );
}

#[test]
fn test_ckb_transfer_not_enough() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500, 0, 0),
        AddressData::new(addr_2, 0, 200, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_1.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_2.to_string(),
                action: Action::PayByFrom,
            },
            amount: 600u128,
        }],
    };

    assert!(engine.rpc().build_transfer_transaction(payload).is_err());
}

// ********************************
// sUDT transfer completion tests
// ********************************
#[test]
fn test_udt_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 400, 10_000, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByFrom,
            },
            amount: 100u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 2, 3, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[2].capacity,
        ((400 - 142 - 5) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 100);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10_000 - 100));
}

#[test]
fn test_list_udt_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 400, 100, 0),
        AddressData::new(addr_3, 0, 500, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string(), addr_3.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByFrom,
            },
            amount: 300u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 3, 3, 2);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(ret.sigs_entry[1].pub_key, addr_3.to_string());
    assert_eq!(ret.sigs_entry[1].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[2].capacity, ((400 - 5) * BYTE_SHANNONS).into());
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 300);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (500 + 100 - 300));
}

#[test]
fn test_cheque_udt_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 400, 10_000, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::LendByFrom,
            },
            amount: 100u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();
    let cheque_config = engine
        .rpc_config
        .get(special_cells::CHEQUE)
        .cloned()
        .unwrap()
        .script;

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 2, 3, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(
        tx_outputs[0].lock.code_hash,
        cheque_config.code_hash().unpack()
    );
    assert_eq!(tx_outputs[0].capacity, (162 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[2].capacity,
        ((400 - 162 - 5) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 100);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10_000 - 100));
}

#[test]
fn test_acp_udt_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 400, 10, 100),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByFrom,
            },
            amount: 50u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 3, 3, 2);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(ret.sigs_entry[1].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[1].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[2].capacity,
        ((400 - 142 - 5) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 50);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10 + 100 - 50));
}

#[test]
fn test_udt_transfer_to_acp_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 20),
        AddressData::new(addr_2, 400, 10000, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByTo,
            },
            amount: 50u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 3, 3, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_2.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[2].capacity, ((400 - 5) * BYTE_SHANNONS).into());
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 50 + 20);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10000 - 50));
}

#[test]
#[ignore]
fn test_udt_with_acp_transfer_to_acp_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyqzse99vquwj6t32xyt6s7p25ymjlslam7s583h63";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 50),
        //AddressData::new(addr_2, 400, 10, 100),
        AddressData::new(addr_3, 400, 10, 50),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_3.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByTo,
            },
            amount: 50u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.build_transfer_transaction(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 4, 3, 2);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_3.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 2);
    assert_eq!(ret.sigs_entry[1].pub_key, addr_3.to_string());
    assert_eq!(ret.sigs_entry[1].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[2].capacity, ((400 - 5) * BYTE_SHANNONS).into());
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), (50 + 50));
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10 + 50 - 50));
}

#[test]
fn test_udt_transfer_udt_not_enough() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 20),
        AddressData::new(addr_2, 0, 10, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 0,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByTo,
            },
            amount: 50u128,
        }],
    };

    let ret = engine.rpc().build_transfer_transaction(payload);
    assert!(ret.is_err());
}

#[test]
fn test_acp_udt_transfer_to_has_no_acp() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        AddressData::new(addr_2, 400, 10, 0),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5 * BYTE_SHANNONS,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByFrom,
            },
            amount: 50u128,
        }],
    };

    let ret = engine.rpc().build_transfer_transaction(payload);
    assert!(ret.is_err());
}

// ********************************
// Generate ACP completion tests
// ********************************
#[test]
fn test_generate_sudt_acp() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 208, 10, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: None,
            min_udt: None,
        }],
    };

    let ret = engine
        .rpc()
        .build_wallet_creation_transaction(payload)
        .unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 1, 2, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_1.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[1].capacity,
        ((208 - 142 - 5) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 0);
}

#[test]
fn test_generate_sudt_acp_with_min() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 210, 10, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: Some(61),
            min_udt: Some(5),
        }],
    };

    let ret = engine
        .rpc()
        .build_wallet_creation_transaction(payload)
        .unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 1, 2, 1);

    assert_eq!(ret.sigs_entry[0].pub_key, addr_1.to_string());
    assert_eq!(ret.sigs_entry[0].group_len, 1);
    assert_eq!(tx_outputs[0].capacity, ((142 + 2) * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[1].capacity,
        ((210 - 142 - 5 - 2) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 0);
}

#[test]
fn test_generate_acp_invalid_info() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 10, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: None,
            min_udt: Some(5),
        }],
    };

    let ret = engine.rpc().build_wallet_creation_transaction(payload);
    assert!(ret.is_err());
}

#[test]
fn test_generate_acp_inexistent_sudt() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: None,
            min_udt: None,
        }],
    };

    let ret = engine.rpc().build_wallet_creation_transaction(payload);
    assert!(ret.is_err());
}

#[test]
fn test_generate_sudt_acp_lack_ckb() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 207, 0, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: None,
            min_udt: None,
        }],
    };

    let ret = engine.rpc().build_wallet_creation_transaction(payload);
    assert!(ret.is_err());
}

#[test]
fn test_generate_sudt_with_min_acp_lack_ckb() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    // let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    // let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 209, 0, 0),
        // AddressData::new(addr_2, 400, 10, 0, 0),
        // AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = CreateWalletPayload {
        ident: addr_1.to_string(),
        fee: 5 * BYTE_SHANNONS,
        info: vec![WalletInfo {
            udt_hash: SUDT_HASH.read().clone(),
            min_ckb: Some(61),
            min_udt: Some(1),
        }],
    };

    let ret = engine.rpc().build_wallet_creation_transaction(payload);
    assert!(ret.is_err());
}
