use super::common::check_amount;
use super::common::post_http_request;
use serde_json::Value;

#[test]
fn test_address() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_sudt_issue_transaction",
        "params": [
            {
                "owner": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9",
                "to": {
                    "to_infos": [
                        {
                            "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                            "amount": "100000000"
                        }
                    ],
                    "mode": "HoldByFrom"
                },
                "pay_fee": null,
                "change": null,
                "fee_rate": 1000,
                "since": null
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    let sudt_output = outputs
        .iter()
        .find(|output| output["type"] != Value::Null)
        .unwrap();
    assert_eq!(
        sudt_output["type"]["code_hash"],
        "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4"
    );
    check_amount(outputs.iter(), 1000000000000, None);
}
