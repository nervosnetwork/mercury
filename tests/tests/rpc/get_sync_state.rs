use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_sync_state() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_sync_state",
        "params": [],
        "id": 100
    }"#);
    let r = &resp["result"];

    assert_ne!(r["value"], Value::Null);
    assert!(r["value"]["current"].as_i64().unwrap() > 0);
    assert!(r["value"]["target"].as_i64().unwrap() > 0);
}
