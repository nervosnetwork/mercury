use super::common::post_http_request;

#[test]
fn test_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0x17fb39e89f7");
}

#[test]
fn test_nonexistent_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde52"
                },
                "script_type": "lock"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0x0");
}

#[test]
fn test_lock_and_type_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock",
                "filter": {
                    "script": {
                        "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
                        "hash_type": "type",
                        "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
                    }
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["capacity"], "0x34e62ce00");
}

#[test]
fn test_type_and_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
                    "hash_type": "type",
                    "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
                },
                "script_type": "type",
                "filter": {
                    "script": {
                        "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                        "hash_type": "type",
                        "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde"
                    }
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["capacity"], "0x34e62ce00");
}

#[test]
fn test_output_data_len_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock",
                "filter": {
                    "output_data_len_range": ["0x0", "0x2"]
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0x17c653bbbf7");
}

#[test]
fn test_output_capacity_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock",
                "filter": {
                    "output_capacity_range": ["0x0", "0x1000000000"]
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0xad9939200");
}

#[test]
fn test_block_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock",
                "filter": {
                    "block_range": ["0x2191c0", "0x219990"]
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0xbe68708fb7");
}

#[test]
fn test_script_len_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells_capacity",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock",
                "filter": {
                    "block_range": ["0x2191c0", "0x219990"],
                    "script_len_range": ["0x0", "0x1"] 
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["capacity"], "0xbe68708fb7");
}
