use super::common::check_amount;
use super::common::post_http_request;

#[test]
fn test_simple_transfer() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_simple_transfer_transaction",
        "params": [
            {
                "asset_info": {
                    "asset_type": "CKB",
                    "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "from": ["ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "9650000000"
                    }
                ],
                "change": null,
                "fee_rate": null,
                "since": null
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(outputs.len(), 2);

    check_amount(outputs.iter(), 2000000000000, None);
    // let receiver_output = outputs.iter().find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712").unwrap();
    // assert_eq!(receiver_output["capacity"], "0x23f2f5080");

    // let witnesses = &tx["witnesses"].as_array().unwrap();
    // assert_eq!(witnesses.len(), 2);
    // assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    // let signature_actions = &r["signature_actions"].as_array().unwrap();
    // assert_eq!(signature_actions.len(), 1);
    // assert_eq!(signature_actions[0]["signature_info"]["algorithm"], "Secp256k1");
    // assert_eq!(signature_actions[0]["signature_info"]["address"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9");
}
