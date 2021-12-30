use super::common::post_http_request;

#[test]
fn test_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 5);
    assert_eq!(cells[0]["block_number"], "0x219817");
    assert_eq!(cells[0]["out_point"]["index"], "0x2");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xf04f3389c99a5d646b4e78b7fb5e1d8e150fa833790519b652194c848b79f533"
    );
    assert_eq!(cells[4]["block_number"], "0x21a37a");
    assert_eq!(cells[4]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[4]["out_point"]["tx_hash"],
        "0xfcb4a627f5ab3cca0b9c77b4878f6379fbb3ddc9c1dbc5feead33284b75f8138"
    );
}

#[test]
fn test_lock_script_desc() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde51"
                },
                "script_type": "lock"
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 5);
    assert_eq!(cells[0]["block_number"], "0x21a37a");
    assert_eq!(cells[0]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xfcb4a627f5ab3cca0b9c77b4878f6379fbb3ddc9c1dbc5feead33284b75f8138"
    );
    assert_eq!(cells[4]["block_number"], "0x219817");
    assert_eq!(cells[4]["out_point"]["index"], "0x2");
    assert_eq!(
        cells[4]["out_point"]["tx_hash"],
        "0xf04f3389c99a5d646b4e78b7fb5e1d8e150fa833790519b652194c848b79f533"
    );
}

#[test]
fn test_nonexistent_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
        "params": [
            {
                "script": {
                    "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                    "hash_type": "type",
                    "args": "0x0c24d18f16e3c43272695e5db006a22cb9ddde52"
                },
                "script_type": "lock"
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 0);
}

#[test]
fn test_type_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
        "params": [
            {
                "script": {
                    "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
                    "hash_type": "type",
                    "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
                },
                "script_type": "type"
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];
    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 5);
    assert_eq!(cells[0]["block_number"], "0x3a67ce");
    assert_eq!(cells[0]["out_point"]["index"], "0x2");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0x3b5c1c80c207bf9ca4b548bbb6477b825a718de91592c04c683e508708b0b71b"
    );
    assert_eq!(cells[4]["block_number"], "0x399435");
    assert_eq!(cells[4]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[4]["out_point"]["tx_hash"],
        "0x8f932cf2c9d1059d2c2ebc9cab6e9bbb34d3b090de309cac5b97de9c4f596c30"
    );
}

#[test]
fn test_lock_and_type_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];
    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0]["block_number"], "0x219993");
    assert_eq!(cells[0]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xeb7ebb26f7c0f7822925001bf2eaea0b00b8f742bde38c30dc3e43efb6947b51"
    );
}

#[test]
fn test_type_and_lock_script() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0]["block_number"], "0x219993");
    assert_eq!(cells[0]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xeb7ebb26f7c0f7822925001bf2eaea0b00b8f742bde38c30dc3e43efb6947b51"
    );
}

#[ignore = "Need fix. Indexer returns sUDT cells while mercury does not."]
#[test]
fn test_output_data_len_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 4);
    assert_eq!(cells[0]["block_number"], "0x21a37a");
    assert_eq!(cells[0]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xfcb4a627f5ab3cca0b9c77b4878f6379fbb3ddc9c1dbc5feead33284b75f8138"
    );
    assert_eq!(cells[4]["block_number"], "0x219817");
    assert_eq!(cells[4]["out_point"]["index"], "0x2");
    assert_eq!(
        cells[4]["out_point"]["tx_hash"],
        "0xf04f3389c99a5d646b4e78b7fb5e1d8e150fa833790519b652194c848b79f533"
    );
}

#[test]
fn test_output_capacity_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[0]["block_number"], "0x21a37a");
    assert_eq!(cells[0]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xfcb4a627f5ab3cca0b9c77b4878f6379fbb3ddc9c1dbc5feead33284b75f8138"
    );
    assert_eq!(cells[2]["block_number"], "0x219993");
    assert_eq!(cells[2]["out_point"]["index"], "0x1");
    assert_eq!(
        cells[2]["out_point"]["tx_hash"],
        "0xeb7ebb26f7c0f7822925001bf2eaea0b00b8f742bde38c30dc3e43efb6947b51"
    );
}

#[test]
fn test_block_range() {
    let resp = post_http_request(
        r#"{
        "id": 2,
        "jsonrpc": "2.0",
        "method": "get_cells",
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
            },
            "desc",
            "0x5"
        ]
    }"#,
    );
    let r = &resp["result"];

    let cells = r["objects"].as_array().unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0]["block_number"], "0x219817");
    assert_eq!(cells[0]["out_point"]["index"], "0x2");
    assert_eq!(
        cells[0]["out_point"]["tx_hash"],
        "0xf04f3389c99a5d646b4e78b7fb5e1d8e150fa833790519b652194c848b79f533"
    );
}
