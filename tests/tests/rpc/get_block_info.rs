use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_block_number() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_block_info",
        "params": [
            {
                "block_number": "0x7c2c1"
            }
        ],
        "id": 100
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["block_number"], "0x7c2c1");
    assert_eq!(
        r["block_hash"],
        "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70"
    );

    let txs = r["transactions"].as_array().unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(
        txs[0]["tx_hash"],
        "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af"
    );
    assert_eq!(txs[0]["timestamp"], "0x174d85f13a0");
}

#[test]
fn test_block_hash() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_block_info",
        "params": [
            {
                "block_hash": "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70"
            }
        ],
        "id": 100
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["block_number"], "0x7c2c1");
    assert_eq!(
        r["block_hash"],
        "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70"
    );

    let txs = r["transactions"].as_array().unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(
        txs[0]["tx_hash"],
        "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af"
    );
    assert_eq!(txs[0]["timestamp"], "0x174d85f13a0");
}

#[test]
fn test_mismatch_block_hash_and_number() {
    // block_hash and block number are inconsistent.
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_block_info",
        "params": [
            {
                "block_hash": "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70",
                "block_number": 108710
            }
        ],
        "id": 100
    }"#,
    );

    assert_ne!(resp["error"], Value::Null);
}
