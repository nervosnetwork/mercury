
use super::common::post_http_request;
use super::common::check_amount;

#[test]
fn test_dao_withdraw_by_address() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_dao_withdraw_transaction",
        "params": [
            {
                "from": {
                    "type": "Address",
                    "value": "ckt1qyqrc4wkvc95f2wxguxaafwtgavpuqnqkxzqs0375w"
                },
                "fee_rate": 1000
            }
        ]
    }"#);
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let _inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    check_amount(outputs.iter(), 810000000000, None);
}
