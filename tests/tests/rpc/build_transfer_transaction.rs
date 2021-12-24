use serde_json::Value;
use super::common::post_http_request;
use super::common::check_amount;

#[test]
fn test_ckb_single_from() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_transfer_transaction",
        "params": [
            {
                "asset_info": {
                    "asset_type": "CKB",
                    "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "from": {
                    "items": [
                        {
                            "type": "Address",
                            "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                        }
                    ],
                    "source": "Free"
                },
                "to": {
                    "to_infos": [
                        {
                            "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                            "amount": "9650000000"
                        }
                    ],
                    "mode": "HoldByFrom"
                },
                "pay_fee": null,
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": 6000000
                }
            }
        ]
    }"#);
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    let receiver_output = outputs.iter().find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712").unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
    check_amount(outputs.iter(), 1000000000000, None);

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    let signature_actions = &r["signature_actions"].as_array().unwrap();
    assert_eq!(signature_actions.len(), 1);
    assert_eq!(signature_actions[0]["signature_info"]["algorithm"], "Secp256k1");
    assert_eq!(signature_actions[0]["signature_info"]["address"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9");
}

#[test]
fn test_udt_single_from() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_transfer_transaction",
        "params": [
            {
                "asset_info": {
                    "asset_type": "UDT",
                    "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                },
                "from": {
                    "items": [
                        {
                            "type": "Address",
                            "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                        }
                    ],
                    "source": "Free"
                },
                "to": {
                    "to_infos": [
                        {
                            "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                            "amount": "5"
                        }
                    ],
                    "mode": "HoldByFrom"
                },
                "pay_fee": null,
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": 6000000
                }
            }
        ]
    }"#);
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    check_amount(outputs.iter(), 1000000000000, None);
}

//199999999947

#[test]
fn test_hold_by_to_no_acp() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_transfer_transaction",
        "params": [
            {
                "asset_info": {
                    "asset_type": "CKB",
                    "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "from": {
                    "items": [
                        {
                            "type": "Address",
                            "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                        }
                    ],
                    "source": "Free"
                },
                "to": {
                    "to_infos": [
                        {
                            "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                            "amount": "9650000000"
                        }
                    ],
                    "mode": "HoldByTo"
                },
                "pay_fee": null,
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": 6000000
                }
            }
        ]
    }"#);
    let r = &resp["result"];
    assert_ne!(resp["error"], Value::Null);
}
