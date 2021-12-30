use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_tip() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_tip",
        "params": [],
        "id": 100
    }"#,
    );
    let r = &resp["result"];

    assert_ne!(r["block_number"], Value::Null);
}
