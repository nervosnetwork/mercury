use super::common::check_amount;
use super::common::post_http_request;
use serde_json::Value;

#[test]
fn test_ckb() {
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

    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");

    check_amount(outputs.iter(), 20199_9999_9470, None);
}

#[test]
fn test_udt() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_simple_transfer_transaction",
        "params": [
            {
                "asset_info": {
                    "asset_type": "UDT",
                    "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                },
                "from": ["ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "5"
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
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    let receiver_output_index = outputs.iter().position(|output| output["lock"]["args"] == "0x7a429a6618d25b6e96260f4ebc870903c0d288a245211d0ce85c7d3228c35536d6db7f4ff547dc68").unwrap();
    assert_eq!(
        tx["outputs_data"][receiver_output_index],
        "0x05000000000000000000000000000000"
    );
    let sender_output_index = outputs
        .iter()
        .position(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(
        tx["outputs_data"][sender_output_index],
        "0x37000000000000000000000000000000"
    );
}

#[test]
fn test_ckb_insufficient_amount() {
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
                        "amount": "9999650000000"
                    }
                ],
                "change": null,
                "fee_rate": null,
                "since": null
            }
        ]
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}
