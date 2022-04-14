use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_existent_transaction() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_transaction_info",
        "params": [
            "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3"
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["status"], "committed");

    let tx = &r["transaction"];
    assert_eq!(
        tx["tx_hash"],
        "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3"
    );

    let records = &tx["records"].as_array().unwrap();
    // input #0
    assert_eq!(records[0]["amount"], "-14367400000");
    assert_eq!(records[0]["occupied"], "0x34e62ce00");
    assert_eq!(records[0]["block_number"], "0x342814");
    assert_eq!(records[0]["epoch_number"], "0x708028c000ca2");

    // output #0
    assert_eq!(records[1]["amount"], "14367200000");
    assert_eq!(records[1]["occupied"], "0x34e62ce00");
    assert_eq!(records[1]["block_number"], "0x3428a9");
    assert_eq!(records[0]["epoch_number"], "0x708028c000ca2");
}

#[test]
fn test_nonexistent_transaction() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_transaction_info",
        "params": [
            "0xd82e3050472d5b5f7603cb8142a57caffdcb2c20bd88577f77da23822d4d42a3"
        ]
    }"#,
    );

    assert_ne!(resp["error"], Value::Null);
}
