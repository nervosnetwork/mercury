use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_address_ckb() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }],
            "tip_block_number": "0x3783ca"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x3783ca");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(
        balance["asset_info"]["udt_hash"],
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(balance["free"], "0x5818b3a230b");
}

#[test]
fn test_address_udt() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
            },
            "asset_infos": [{
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": "0x397d3a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x397d3a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(
        balance["asset_info"]["udt_hash"],
        "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    );
    assert_eq!(balance["free"], "0x3c");
    assert_eq!(balance["occupied"], "0x0");
    assert_eq!(balance["frozen"], "0x0");
}

#[test]
fn test_address_all() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
            },
            "asset_infos": [],
            "tip_block_number": "0x397d3a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x397d3a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let (udt_balance, ckb_balance) = if balances[0]["asset_info"]["asset_type"] == "UDT" {
        (&balances[0], &balances[1])
    } else {
        (&balances[1], &balances[0])
    };

    assert_eq!(udt_balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(
        udt_balance["asset_info"]["udt_hash"],
        "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    );
    assert_eq!(udt_balance["free"], "0x3c");
    assert_eq!(udt_balance["occupied"], "0x0");
    assert_eq!(udt_balance["frozen"], "0x0");

    assert_eq!(ckb_balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(
        ckb_balance["asset_info"]["udt_hash"],
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn test_address_cheque() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4kaedejfkzfry4ccapp22qgsfr6schlz7aj5lc09uvu8xw3g7jg8x747xgl6jnet87rser4k"
            },
            "asset_infos": [],
            "tip_block_number": "0x397d3a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x397d3a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let (udt_balance, ckb_balance) = if balances[0]["asset_info"]["asset_type"] == "UDT" {
        (&balances[0], &balances[1])
    } else {
        (&balances[1], &balances[0])
    };

    assert_eq!(udt_balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(
        udt_balance["asset_info"]["udt_hash"],
        "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    );
    assert_eq!(udt_balance["free"], "0x384");
    assert_eq!(udt_balance["occupied"], "0x0");
    assert_eq!(udt_balance["frozen"], "0x0");

    assert_eq!(ckb_balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(
        ckb_balance["asset_info"]["udt_hash"],
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(ckb_balance["free"], "0x0");
    assert_eq!(ckb_balance["occupied"], "0x21f25b7200");
    assert_eq!(ckb_balance["frozen"], "0x0");
}

#[test]
fn test_identity_ckb() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }],
            "tip_block_number": "0x39832a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x39832a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);

    let balance = balances
        .iter()
        .find(|balance| balance["asset_info"]["asset_type"] == "CKB")
        .unwrap();
    assert_eq!(balance["free"], "0x2b72f8c5fee");
}

#[test]
fn test_identity_udt() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": "0x39832a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x39832a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0]["asset_info"]["asset_type"], "UDT");
    assert_eq!(
        balances[0]["asset_info"]["udt_hash"],
        "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    );
}

#[test]
fn test_identity_all() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [],
            "tip_block_number": "0x39832a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x39832a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let ckb_balance = balances
        .iter()
        .find(|balance| balance["asset_info"]["asset_type"] == "CKB")
        .unwrap();
    assert_eq!(ckb_balance["free"], "0x2b72f8c5fee");
    assert_eq!(ckb_balance["occupied"], "0x34e62ce00");

    let udt_balance = balances
        .iter()
        .find(|balance| balance["asset_info"]["asset_type"] == "UDT")
        .unwrap();
    assert_eq!(udt_balance["free"], "0x82");
}

#[test]
fn test_identity_multiple_assets() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
            "type": "Identity",
            "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": "0x39832a"
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], "0x39832a");

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let acp_ckb_balance = balances
        .iter()
        .find(|balance| balance["asset_info"]["asset_type"] == "CKB")
        .unwrap();
    assert_eq!(acp_ckb_balance["free"], "0x2b72f8c5fee");
    assert_eq!(acp_ckb_balance["occupied"], "0x34e62ce00");

    let acp_udt_balance = balances
        .iter()
        .find(|balance| balance["asset_info"]["asset_type"] == "UDT")
        .unwrap();
    assert_eq!(acp_udt_balance["free"], "0x82");
}

#[test]
fn test_out_point() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "OutPoint",
                "value": {
                    "tx_hash": "0xea0b230104fd3be2cc33ab50c3d591dc6cefbe8ed83f7e63c8142de4b5a0ee72",
                    "index": "0x0"
                }
            },
            "asset_infos": []
        }],
        "id": 10
    }"#,
    );
    let r = &resp["result"];

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(balance["free"], "0x2d5538d9c5");
}

#[test]
fn test_out_point_spent_cheque() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "OutPoint",
                "value": {
                    "tx_hash": "0x0c9fe78130502bcd53656f6224072bd44b4ab357ba7351e1f37e72d4f12b07b9",
                    "index": "0x0"
                }
            },
            "asset_infos": []
        }],
        "id": 10
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}

#[test]
fn test_out_point_cheque_ckb() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "OutPoint",
                "value": {
                    "tx_hash": "0x52b1cf0ad857d53e1a3552944c1acf268f6a6aea8e8fc85fe8febcb8127d56f0",
                    "index": "0x0"
                }
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }]
        }],
        "id": 10
    }"#,
    );

    let r = &resp["result"];

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(balance["free"], "0x0");
    assert_eq!(balance["occupied"], "0x3c5986200");
}

#[test]
fn test_out_point_cheque_udt() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "OutPoint",
                "value": {
                    "tx_hash": "0x52b1cf0ad857d53e1a3552944c1acf268f6a6aea8e8fc85fe8febcb8127d56f0",
                    "index": "0x0"
                }
            },
            "asset_infos": [{
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }]
        }],
        "id": 10
    }"#,
    );

    let r = &resp["result"];

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(balance["free"], "0x64");
    assert_eq!(balance["occupied"], "0x0");
}

#[test]
fn test_illegal_address() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1q"
            },
            "asset_infos": [],
            "tip_block_number": 3636218
        }],
        "id": 100
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}
