use super::common::check_amount;
use super::common::post_http_request;
use serde_json::Value;

#[test]
fn test_ckb_single_from_single_to() {
    let resp = post_http_request(
        r#"{
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
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
    check_amount(outputs.iter(), 1000000000000, None);

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    let signature_actions = &r["signature_actions"].as_array().unwrap();
    assert_eq!(signature_actions.len(), 1);
    assert_eq!(
        signature_actions[0]["signature_info"]["algorithm"],
        "Secp256k1"
    );
    assert_eq!(signature_actions[0]["signature_info"]["address"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9");
}

#[test]
fn test_ckb_single_from_multiple_to() {
    let resp = post_http_request(
        r#"{
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
                            "amount": "70000000000"
                        },
                        {
                            "address": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqfqyerlanzmnkxtmd9ww9n7gr66k8jt4tclm9jnk",
                            "amount": "20000000000"
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
    }"#,
    );

    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 3);

    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x104c533c00");
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x202647fecc5b9d8cbdb4ae7167e40f5ab1e4baaf")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x4a817c800");
    check_amount(outputs.iter(), 1000000000000, None);

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    let signature_actions = &r["signature_actions"].as_array().unwrap();
    assert_eq!(signature_actions.len(), 1);
    assert_eq!(
        signature_actions[0]["signature_info"]["algorithm"],
        "Secp256k1"
    );
    assert_eq!(signature_actions[0]["signature_info"]["address"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9");
}

#[test]
fn test_ckb_multiple_from_single_to() {
    let resp = post_http_request(
        r#"{
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
                        },
                        {
                            "type": "Address",
                            "value": "ckt1qyp05g42p2h32knvs9nrf3s4zgzxkzyjxygs4e29ue"
                        }
                    ],
                    "source": "Free"
                },
                "to": {
                    "to_infos": [
                        {
                            "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                            "amount": "2001500000000"
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
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let outputs = &tx["outputs"].as_array().unwrap();

    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x1d202b24f00");
}

#[test]
fn test_udt_single_from_single_to() {
    let resp = post_http_request(
        r#"{
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
        "0x9b000000000000000000000000000000"
    );
}

#[test]
fn test_ckb_hold_by_to() {
    let resp = post_http_request(
        r#"{
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
    }"#,
    );
    assert_ne!(resp["error"], Value::Null); // need acp cell
}

#[test]
fn test_ckb_pay_with_acp() {
    let resp = post_http_request(
        r#"{
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
                    "mode": "PayWithAcp"
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
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);  // Unsupport transfer mode: PayWithAcp when transfer CKB
}

#[test]
fn test_udt_pay_with_acp() {
    let resp = post_http_request(
        r#"{
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
                    "mode": "PayWithAcp"
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
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 2);

    let receiver_output_index = outputs.iter().position(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712").unwrap();
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
        "0x9b000000000000000000000000000000"
    );
}

#[test]
fn test_ckb_pay_fee() {
    let resp = post_http_request(
        r#"{
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
                "pay_fee": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": 6000000
                }
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let inputs = &tx["inputs"].as_array().unwrap();
    let outputs = &tx["outputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(outputs.len(), 3);

    let sender_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(sender_output["capacity"], "0xe69575bf80");
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
    let pay_fee_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x3f1573b44218d4c12a91919a58a863be415a2bc3");
    assert_ne!(pay_fee_output, None);
}

#[test]
fn test_ckb_change() {
    let resp = post_http_request(
        r#"{
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
                "change": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70",
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": 6000000
                }
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

    let sender_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111");
    // .unwrap();
    // assert_eq!(sender_output["capacity"], "0xe69575bf80");
    assert_eq!(sender_output, None);
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
    let pay_fee_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x3f1573b44218d4c12a91919a58a863be415a2bc3");
    assert_ne!(pay_fee_output, None);
}

#[test]
fn test_ckb_single_from_single_to_identity() {
    let resp = post_http_request(
        r#"{
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
                            "type": "Identity",
                            "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
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
    }"#,
    );
    let r = &resp["result"];
    let tx = &r["tx_view"];

    let outputs = &tx["outputs"].as_array().unwrap();
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
}
