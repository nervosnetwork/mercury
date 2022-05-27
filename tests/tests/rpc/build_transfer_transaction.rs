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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "0x23f2f5080"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "absolute",
                    "type_": "block_number",
                    "value": "0x5b8d80"
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

    let script_groups = &r["script_groups"].as_array().unwrap();
    assert_eq!(script_groups.len(), 1);
    assert_eq!(script_groups[0]["group_type"], "Lock");
    assert_eq!(
        script_groups[0]["script"]["args"],
        "0xfa22aa0aaf155a6c816634c61512046b08923111"
    );
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "0x104c533c00"
                    },
                    {
                        "address": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqfqyerlanzmnkxtmd9ww9n7gr66k8jt4tclm9jnk",
                        "amount": "0x4a817c800"
                    }
                ],
                "output_capacity_provider": "from",
                "pay_fee": "from",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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

    let script_groups = &r["script_groups"].as_array().unwrap();
    assert_eq!(script_groups.len(), 1);
    assert_eq!(script_groups[0]["group_type"], "Lock");
    assert_eq!(
        script_groups[0]["script"]["args"],
        "0xfa22aa0aaf155a6c816634c61512046b08923111"
    );
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    },
                    {
                        "type": "Address",
                        "value": "ckt1qyp05g42p2h32knvs9nrf3s4zgzxkzyjxygs4e29ue"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "0x1d202b24f00"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qqdpunl0xn6es2gx7azmqj870vggjer7sg6xqa8q7vkzan3xea43uqt6g2dxvxxjtdhfvfs0f67gwzgrcrfg3gj9yywse6zu05ez3s64xmtdkl6074rac6q3f7cvk",
                        "amount": "0x5"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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
fn test_ckb_to_provide_capacity() {
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "9650000000"
                    }
                ],
                "output_capacity_provider": null,
                "pay_fee": null,
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
                }
            }
        ]
    }"#,
    );
    assert_ne!(resp["error"], Value::Null); // need acp cell
}

#[test]
fn test_udt_from_provide_capacity_to_acp_lock() {
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qp3g8fre50846snkekf4jn0f7xp84wd4t3astv7j3exzuznfdnl06qv6e65dqy3kfslr3j2cdh4enhyqeqyawysf7sf4c",
                        "amount": "0x5"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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

    let receiver_output_index = outputs
        .iter()
        .position(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(
        tx["outputs_data"][receiver_output_index],
        "0x05000000000000000000000000000000"
    );
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x34e62ce00");

    let sender_output_index = outputs
        .iter()
        .position(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(
        tx["outputs_data"][sender_output_index],
        "0x9b000000000000000000000000000000"
    );
    let sender_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(sender_output["capacity"], "0xea2e5a052f");

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
}

#[test]
fn test_udt_from_provide_capacity_to_pw_lock() {
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qpvvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdxxqdd40lmnsnukjh3qr88hjnfqvc4yg8g0gskp8ffv",
                        "amount": "0x5"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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

    let receiver_output_index = outputs
        .iter()
        .position(|output| output["lock"]["args"] == "0xadabffb9c27cb4af100ce7bca6903315220e87a2")
        .unwrap();
    assert_eq!(
        tx["outputs_data"][receiver_output_index],
        "0x05000000000000000000000000000000"
    );
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0xadabffb9c27cb4af100ce7bca6903315220e87a2")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x34e62ce00");

    let sender_output_index = outputs
        .iter()
        .position(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(
        tx["outputs_data"][sender_output_index],
        "0x9b000000000000000000000000000000"
    );
    let sender_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(sender_output["capacity"], "0xea2e5a052f");

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "0x23f2f5080"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "to",
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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
        .find(|output| output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .unwrap();
    assert_eq!(sender_output["capacity"], "0xe69575bf80");
    let receiver_output = outputs
        .iter()
        .find(|output| output["lock"]["args"] == "0x9acea8d012364c3e38c9586deb99dc80c809d712")
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f4eb0");
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
                "from": [
                    {
                        "type": "Identity",
                        "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld",
                        "amount": "0x23f2f5080"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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

#[test]
fn test_ckb_single_from_single_to_any_address() {
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
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qq56s9rnufyjfcu54y2g4v6hcfyjlm0k2fqcfzm6safe5ldec02r7qyz366atsthnl87z038a6lcqaqsq3gdkdd43kj5emkwvdd50astp578twzk",
                        "amount": "0x23f2f5080"
                    }
                ],
                "output_capacity_provider": "From",
                "pay_fee": "From",
                "change": null,
                "fee_rate": null,
                "since": {
                    "flag": "Absolute",
                    "type_": "BlockNumber",
                    "value": "0x5b8d80"
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
        .find(|output| {
            output["lock"]["args"]
                == "0x828eb5d5c1779fcfe13e27eebf8074100450db35b58da54ceece635b47f60b0d"
        })
        .unwrap();
    assert_eq!(receiver_output["capacity"], "0x23f2f5080");
    check_amount(outputs.iter(), 1000000000000, None);

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    let script_groups = &r["script_groups"].as_array().unwrap();
    assert_eq!(script_groups.len(), 1);
    assert_eq!(script_groups[0]["group_type"], "Lock");
    assert_eq!(
        script_groups[0]["script"]["args"],
        "0xfa22aa0aaf155a6c816634c61512046b08923111"
    );
}
