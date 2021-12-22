use super::common::post_http_request;

#[test]
fn test_testnet() {
    let resp = post_http_request(r#"{
            "jsonrpc": "2.0",
            "method": "get_mercury_info",
            "params": [],
            "id": 100
        }"#);
    let r = &resp["result"];

    assert_eq!(r["network_type"], "Testnet");
}
