use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_structure_type_native() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112",
                "index": "0x2"
            },
            "structure_type": "Native"
        }]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["value"]["tx_status"]["status"], "committed");
    assert_eq!(
        r["value"]["tx_status"]["block_hash"],
        "0x54af022e0fdebed3690ee6d6e2368f3c976c6ad6cfcdbfa307869fd57c2d5129"
    );

    let tx = &r["value"]["transaction"];
    assert_eq!(
        tx["hash"],
        "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1"
    );

    let inputs = &tx["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(
        inputs[1]["previous_output"]["tx_hash"],
        "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112"
    );
    assert_eq!(inputs[1]["previous_output"]["index"], "0x2");

    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(outputs.len(), 3);

    let output = outputs
        .iter()
        .find(|outputs| outputs["capacity"] == "0x7efc5e6418")
        .unwrap();
    assert_eq!(
        output["lock"]["args"],
        "0xa3b8598e1d53e6c5e89e8acb6b4c34d3adb13f2b"
    );
    assert_eq!(output["type"], Value::Null);

    let outputs_data = &tx["outputs_data"].as_array().unwrap();
    assert_eq!(outputs_data.len(), 3);
}

#[test]
fn test_structure_type_double_entry() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112",
                "index": "0x2"
            },
            "structure_type": "DoubleEntry"
        }]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["type"], "TransactionInfo");

    let tx = &r["value"];
    assert_eq!(
        r["value"]["tx_hash"],
        "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1"
    );

    let records = tx["records"].as_array().unwrap();
    assert_eq!(records.len(), 8);

    // input #1
    let _record = records
        .iter()
        .find(|record| record["amount"] == "0x82c1f6c9d6")
        .unwrap();

    // output #0
    let _record = records
        .iter()
        .find(|record| record["amount"] == "0x64")
        .unwrap();

    // output #2
    let _record = records
        .iter()
        .find(|record| record["amount"] == "0x7efc5e6418")
        .unwrap();
}

#[test]
fn test_structure_type_native_unspent_outpoint() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
                "index": "0x1"
            },
            "structure_type": "Native"
        }]
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}

#[test]
fn test_structure_type_native_nonexistent_outpoint() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0x1f8c79eb3671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
                "index": "0x1"
            },
            "structure_type": "Native"
        }]
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}
