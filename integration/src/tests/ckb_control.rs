use super::IntegrationTest;
use crate::utils::const_definition::CKB_URI;
use crate::utils::post_http_request;

use serde_json::Value;

fn test_generate_block() {
    let resp = post_http_request(
        CKB_URI,
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "generate_block",
        "params": [
            null, null
            ]
        }"#,
    );
    assert_eq!(resp["error"], Value::Null);
}

inventory::submit!(IntegrationTest {
    name: "test_generate_block",
    test_fn: test_generate_block
});
