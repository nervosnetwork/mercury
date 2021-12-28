use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_query_by_address() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], 4);
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 4);

    txs.iter().for_each(|tx| assert_eq!(tx["type"], "TransactionInfo"));

    let tx = &txs[0]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(tx["tx_hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    assert_eq!(records.len(), 5);
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqflz4emgssc6nqj4yv3nfv2sca7g9dzhscgmg28x").is_some());
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn").is_some());

    let tx = &txs[1]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(tx["tx_hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(records.len(), 16);
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn").is_some());

    let tx = &txs[2]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(tx["tx_hash"], "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3");
    assert_eq!(records.len(), 12);
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn").is_some());


    let tx = &txs[3]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(tx["tx_hash"], "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f");
    assert_eq!(records.len(), 3);
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn").is_some());
}

#[test]
fn test_query_by_address_native() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "Native"
            }
        ]
    }"#);
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], 4);
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 4);

    txs.iter().for_each(|tx| assert_eq!(tx["type"], "TransactionWithRichStatus"));

    let tx = &txs[0]["value"];
    assert_eq!(tx["tx_status"]["block_hash"], "0xb12c682135282fe03b2d5c2c1f11f3add67c8f88e1b1914dfa1fc67c30dbb107");
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(tx["transaction"]["hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 1);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 4);

    let tx = &txs[1]["value"];
    assert_eq!(tx["tx_status"]["block_hash"], "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538");
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(tx["transaction"]["hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 4);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 4);
}


#[test]
fn test_query_by_address_asc() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "order": "asc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs[3]["value"]["tx_hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    assert_eq!(txs[2]["value"]["tx_hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(txs[1]["value"]["tx_hash"], "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3");
    assert_eq!(txs[0]["value"]["tx_hash"], "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f");
}

#[test]
fn test_query_by_address_ckb() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [
                    {
                        "asset_type": "CKB",
                        "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                    }
                ],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], 1);
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 1);

    let tx = &txs[0]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(tx["tx_hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    assert_eq!(records.len(), 5);
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqflz4emgssc6nqj4yv3nfv2sca7g9dzhscgmg28x").is_some());
    assert!(records.iter().find(|r| r["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn").is_some());
}

#[test]
fn test_query_by_address_udt() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [
                    {
                        "asset_type": "UDT",
                        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                    }
                ],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], 3);
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 3);

    assert_eq!(txs[0]["value"]["tx_hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(txs[1]["value"]["tx_hash"], "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3");
    assert_eq!(txs[2]["value"]["tx_hash"], "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f");
}

// TODO: Can not find acp tx when querying by identity. Need fix
#[ignore]
#[test]
fn test_query_by_identity_ckb() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Identity",
                    "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
                },
                "asset_infos": [
                    {
                        "asset_type": "CKB",
                        "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                    }
                ],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let _r = &resp["result"];
}

// TODO: Fal to find corresponding tx. Need fix
#[ignore]
#[test]
fn test_query_by_identity_udt() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Identity",
                    "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
                },
                "asset_infos": [
                    {
                        "asset_type": "UDT",
                        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                    }
                ],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let _r = &resp["result"];
}


// TODO: The next_cursor is not Null. Need fix
#[test]
fn test_query_by_record() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Record",
                    "value": "3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed0000000000636b7431717136706e6777716e366539766c6d393274683834726b306c346a703268386c757263686a6d6e7776386b71337274357073663476713036793234713474633474666b677a6533356363323379707274707a66727a79677370746b7a6e"
                },
                "asset_infos": [
                    {
                        "asset_type": "UDT",
                        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
                    }
                ],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "Native"
            }
        ]
    }"#);
    let r = &resp["result"];

    // assert_eq!(r["next_cursor"], Value::Null); // TODO: The next_cursor is not Null. Need fix.
    assert_eq!(r["count"], 1);
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 1);

    txs.iter().for_each(|tx| assert_eq!(tx["type"], "TransactionWithRichStatus"));

    let tx = &txs[0]["value"];
    assert_eq!(tx["tx_status"]["block_hash"], "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538");
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(tx["transaction"]["hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 4);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 4);
}

// TODO: Filter extra.Dao doesn't work. All txs are turned. Need fix.
#[ignore]
#[test]
fn test_query_by_extra_dao() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qyqrc4wkvc95f2wxguxaafwtgavpuqnqkxzqs0375w"
                },
                "asset_infos": [],
                "extra": "Dao",
                "block_range": [0, 30000],
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 50,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let _r = &resp["result"];
}

#[test]
fn test_query_by_pagination_limit() {
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
                    "order": "desc",
                    "limit": 2,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 2);

    assert_eq!(txs[0]["value"]["tx_hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    assert_eq!(txs[1]["value"]["tx_hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    // assert_eq!(txs[1]["value"]["tx_hash"], "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3");
    // assert_eq!(txs[0]["value"]["tx_hash"], "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f");
}

// `test_query_by_pagination_limit` returns tx 1,2. `test_query_by_pagination_limit` should return 3,4 while it returns 2,3.
// TODO: Need fix
#[ignore]
#[test]
fn test_query_by_pagination_cursor() {
    // cursor comes from case `test_query_by_pagination_limit`
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "Address",
                    "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
                },
                "asset_infos": [],
                "extra": null,
                "block_range": null,
                "pagination": {
                    "cursor": [0,57,125,57,0,0,0,59],
                    "order": "desc",
                    "limit": 2,
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#);
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 2);

    // assert_eq!(txs[0]["value"]["tx_hash"], "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996");
    // assert_eq!(txs[1]["value"]["tx_hash"], "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed");
    assert_eq!(txs[0]["value"]["tx_hash"], "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3");
    assert_eq!(txs[1]["value"]["tx_hash"], "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f");
}
