use serde_json::Value;

use super::common::check_amount;
use super::common::post_http_request;

#[test]
fn test_ckb() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_adjust_account_transaction",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                },
                "from": [],
                "asset_info": {
                    "asset_type": "CKB",
                    "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "account_number": null,
                "extra_ckb": null,
                "fee_rate": null
            }
        ]
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}

#[test]
fn test_udt() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_adjust_account_transaction",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                },
                "from": [],
                "asset_info": {
                    "asset_type": "UDT",
                    "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                },
                "account_number": null,
                "extra_ckb": null,
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
    assert_eq!(outputs.len(), 1);

    check_amount(outputs.iter(), 1000000000000, None);
    // Need output a new acp
    let acp_output = outputs
        .iter()
        .find(|output| {
            output["lock"]["code_hash"]
                == "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356"
                && output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111"
        })
        .unwrap();
    assert_eq!(
        acp_output["type"]["code_hash"],
        "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4"
    );
    assert_eq!(
        acp_output["type"]["args"],
        "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
    );
}

#[test]
fn test_udt_account_number() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_adjust_account_transaction",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                },
                "from": [],
                "asset_info": {
                    "asset_type": "UDT",
                    "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                },
                "account_number": 2,
                "extra_ckb": null,
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
    // Need output a new acp
    let acp_output = outputs
        .iter()
        .find(|output| {
            output["lock"]["code_hash"]
                == "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356"
                && output["lock"]["args"] == "0xfa22aa0aaf155a6c816634c61512046b08923111"
        })
        .unwrap();
    assert_eq!(
        acp_output["type"]["code_hash"],
        "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4"
    );
    assert_eq!(
        acp_output["type"]["args"],
        "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
    );
}

#[ignore = "Need Fix. extra_ckb does not work"]
#[test]
fn test_extra_ckb() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_adjust_account_transaction",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                },
                "from": [],
                "asset_info": {
                    "asset_type": "UDT",
                    "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                },
                "account_number": 2,
                "extra_ckb": 10000,
                "fee_rate": null
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let _tx = &r["tx_view"];
}
