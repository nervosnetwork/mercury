use super::common::check_amount;
use super::common::post_http_request;

#[test]
fn test_dao_deposit_by_address() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_dao_deposit_transaction",
        "params": [
            {
                "from": {
                    "items": [
                        {
                            "type": "Address",
                            "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                        }
                    ],
                    "source": "Free"
                },
                "to": null,
                "amount": "0x4a817c800",
                "fee_rate": null
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

    check_amount(outputs.iter(), 1019999999470, None);
    // Dao output
    assert!(outputs.iter().any(|output| output["lock"]["code_hash"]
        == "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"));
    // Change back to input
    assert!(outputs.iter().any(|output| output["lock"]["code_hash"]
        == "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356"
        && output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111"));

    let witnesses = &tx["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0], "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

    let script_groups = &r["script_groups"].as_array().unwrap();
    assert_eq!(script_groups.len(), 3);
    let script_group = script_groups
        .iter()
        .find(|s| s["group_type"] == "LockScript")
        .unwrap();
    assert_eq!(
        script_group["script"]["args"],
        "0xfa22aa0aaf155a6c816634c61512046b08923111"
    );
}
