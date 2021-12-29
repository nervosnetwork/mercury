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
                "amount": 20000000000,
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

    check_amount(outputs.iter(), 1000000000000, None);
    // Dao output
    assert!(outputs
        .iter()
        .find(|output| output["lock"]["code_hash"]
            == "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8")
        .is_some());
    // Change back to input
    assert!(outputs
        .iter()
        .find(|output| output["lock"]["code_hash"]
            == "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356"
            && output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111")
        .is_some());

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
