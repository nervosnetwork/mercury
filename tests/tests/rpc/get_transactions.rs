use super::common::post_http_request;

#[test]
fn test_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_transactions",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock"
            },
            "asc",
            "0x64"
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = r["objects"].as_array().unwrap();

    assert_eq!(txs.len(), 100);
    assert_eq!(txs[0]["block_number"], "0x1e9174");
    assert_eq!(
        txs[0]["tx_hash"],
        "0x47f48a5f31401d17ebf6d22e8702eea8ffb29cf18c8f74f256155b06e65f5992"
    );
    assert_eq!(txs[0]["tx_index"], "0x2");
    assert_eq!(txs[99]["block_number"], "0x219817");
    assert_eq!(
        txs[99]["tx_hash"],
        "0xf04f3389c99a5d646b4e78b7fb5e1d8e150fa833790519b652194c848b79f533"
    );
    assert_eq!(txs[99]["tx_index"], "0x3");
}

#[test]
fn test_lock_script_desc() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_transactions",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde"
                },
                "script_type": "lock"
            },
            "desc",
            "0x64"
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = r["objects"].as_array().unwrap();

    assert_eq!(txs.len(), 100);
    assert_eq!(txs[0]["block_number"], "0x21a37a");
    assert_eq!(txs[99]["block_number"], "0x1e94bc");
}

#[test]
fn test_type_and_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_transactions",
        "params": [
            {
                "script": {
                    "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
                    "hash_type": "type",
                    "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28"
                },
                "script_type": "type",
                "filter": {
                    "script": {
                        "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                        "hash_type": "type",
                        "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                    }
                }
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];
    let txs = r["objects"].as_array().unwrap();

    assert_eq!(txs.len(), 5);
}
