use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_query_by_address() {
    let resp = post_http_request(
        r#"{
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
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x9");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 9);

    txs.iter()
        .for_each(|tx| assert_eq!(tx["type"], "TransactionInfo"));

    let tx = &txs[0]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(
        tx["tx_hash"],
        "0x87c625edfebb027751e31d416e6408a9628e32ef448eab33819df3e8ed06c312"
    );
    assert_eq!(records.len(), 5);

    let tx = &txs[6]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(
        tx["tx_hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(records.len(), 16);

    let tx = &txs[7]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(
        tx["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(records.len(), 12);

    let tx = &txs[8]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(
        tx["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
    assert_eq!(records.len(), 3);
}

#[test]
fn test_query_by_address_native() {
    let resp = post_http_request(
        r#"{
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
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "Native"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x9");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 9);

    txs.iter()
        .for_each(|tx| assert_eq!(tx["type"], "TransactionWithRichStatus"));

    let tx = &txs[0]["value"];
    assert_eq!(
        tx["tx_status"]["block_hash"],
        "0xaab941604733841bf9f3d7404b5d19d820f33c9ba5c92861410c2cfeac892fb6"
    );
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(
        tx["transaction"]["hash"],
        "0x87c625edfebb027751e31d416e6408a9628e32ef448eab33819df3e8ed06c312"
    );
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 3);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 2);

    let tx = &txs[6]["value"];
    assert_eq!(
        tx["tx_status"]["block_hash"],
        "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538"
    );
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(
        tx["transaction"]["hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 4);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 4);
}

#[test]
fn test_query_by_address_asc() {
    let resp = post_http_request(
        r#"{
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
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(
        txs[3]["value"]["tx_hash"],
        "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
    );
    assert_eq!(
        txs[2]["value"]["tx_hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(
        txs[1]["value"]["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(
        txs[0]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
}

#[test]
fn test_query_by_address_ckb() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": "0xffffffffffffffff",
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x3");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 3);

    let tx = &txs[0]["value"];
    let records = &tx["records"].as_array().unwrap();
    assert_eq!(
        tx["tx_hash"],
        "0x87c625edfebb027751e31d416e6408a9628e32ef448eab33819df3e8ed06c312"
    );
    assert_eq!(records.len(), 5);
}

#[test]
fn test_query_by_address_udt() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x6");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 6);

    assert_eq!(
        txs[3]["value"]["tx_hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(
        txs[4]["value"]["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(
        txs[5]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
}

#[test]
fn test_query_by_identity_ckb() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x7");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 7);

    assert_eq!(
        txs[4]["value"]["tx_hash"],
        "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
    );
    assert_eq!(
        txs[5]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
    assert_eq!(
        txs[6]["value"]["tx_hash"],
        "0x83bc7b8b8936b016b98dfd489a535f6cf7c3d87e60e53f83cc69e8f50c9f30fa"
    );
}

#[test]
fn test_query_by_identity_udt() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0xc");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 12);

    assert_eq!(
        txs[6]["value"]["tx_hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(
        txs[7]["value"]["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(
        txs[8]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
    assert_eq!(
        txs[9]["value"]["tx_hash"],
        "0x0c9fe78130502bcd53656f6224072bd44b4ab357ba7351e1f37e72d4f12b07b9"
    );
    assert_eq!(
        txs[10]["value"]["tx_hash"],
        "0x1256aa76ef4dd7805ad4b1cf9efe87211bd3cdb5dae0e440c29ce4a0db73ea41"
    );
    assert_eq!(
        txs[11]["value"]["tx_hash"],
        "0x88e03bf37db9770a0e496a98bc17cdc31095392a169f0d416bad07b9c58b3501"
    );
}

#[test]
fn test_query_by_out_point() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "OutPoint",
                    "value": {
                        "tx_hash": "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed",
                        "index": "0x0"
                    }
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
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "Native"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x2");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 2);

    txs.iter()
        .for_each(|tx| assert_eq!(tx["type"], "TransactionWithRichStatus"));

    let tx = &txs[1]["value"];
    assert_eq!(
        tx["tx_status"]["block_hash"],
        "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538"
    );
    assert_eq!(tx["tx_status"]["status"], "committed");
    assert_eq!(
        tx["transaction"]["hash"],
        "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
    );
    assert_eq!(tx["transaction"]["inputs"].as_array().unwrap().len(), 4);
    assert_eq!(tx["transaction"]["outputs"].as_array().unwrap().len(), 4);
}

#[test]
fn test_query_by_extra_dao() {
    let resp = post_http_request(
        r#"{
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
                "block_range": ["0x0", "0x7530"],
                "pagination": {
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(&r["count"], "0x5");
    assert_eq!(5, txs.len());
}

#[test]
fn test_query_by_extra_cellbase() {
    let resp = post_http_request(
        r#"{
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
                "extra": "CellBase",
                "block_range": null,
                "pagination": {
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(0, txs.len());
    assert_eq!(&r["count"], "0x0");
}

#[test]
fn test_query_by_out_point_extra_cellbase() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "query_transactions",
        "params": [
            {
                "item": {
                    "type": "OutPoint",
                    "value": {
                        "tx_hash": "0xfc43d8bdfff3051f3c908cd137e0766eecba4e88ae5786760c3e0e0f1d76c004",
                        "index": "0x0"
                    }
                },
                "asset_infos": [],
                "extra": "CellBase",
                "block_range": null,
                "pagination": {
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(0, txs.len());
    assert_eq!(&r["count"], "0x0");
}

#[test]
fn test_query_by_pagination_limit() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": null,
                    "order": "desc",
                    "limit": "0x7",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 7);

    assert_eq!(
        txs[0]["value"]["tx_hash"],
        "0x87c625edfebb027751e31d416e6408a9628e32ef448eab33819df3e8ed06c312"
    );
    assert_eq!(
        txs[1]["value"]["tx_hash"],
        "0x38ac289a3a7529847a25d6845a12b74c11c165e5267b60762e7f8a5cd86fdedf"
    );
    assert_eq!(r["next_cursor"], "0x397d3900000037");
}

#[test]
fn test_query_by_pagination_cursor() {
    // cursor comes from case `test_query_by_pagination_limit`
    let resp = post_http_request(
        r#"{
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
                    "cursor": "0x397d3900000037",
                    "order": "desc",
                    "limit": "0x7",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(r["count"], "0x9");

    assert_eq!(
        txs[0]["value"]["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(
        txs[1]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
    assert_eq!(r["next_cursor"], Value::Null);
}

#[test]
fn test_query_by_pagination_limit_asc() {
    let resp = post_http_request(
        r#"{
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
                    "cursor": null,
                    "order": "asc",
                    "limit": "0x7",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 7);

    assert_eq!(
        txs[0]["value"]["tx_hash"],
        "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
    );
    assert_eq!(
        txs[1]["value"]["tx_hash"],
        "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
    );
    assert_eq!(r["next_cursor"], "0x3dea4d0000000a");
}

#[test]
fn test_query_by_pagination_cursor_asc() {
    // cursor comes from case `test_query_by_pagination_limit`
    let resp = post_http_request(
        r#"{
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
                    "cursor": "0x3dea4d0000000a",
                    "order": "asc",
                    "limit": "0x7",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(r["count"], "0x9");

    assert_eq!(
        txs[0]["value"]["tx_hash"],
        "0x38ac289a3a7529847a25d6845a12b74c11c165e5267b60762e7f8a5cd86fdedf"
    );
    assert_eq!(
        txs[1]["value"]["tx_hash"],
        "0x87c625edfebb027751e31d416e6408a9628e32ef448eab33819df3e8ed06c312"
    );
    assert_eq!(r["next_cursor"], Value::Null);
}

#[test]
fn test_query_by_address_with_block_range() {
    let resp = post_http_request(
        r#"{
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
                "block_range": ["0x397f2e", "0x397f31"],
                "pagination": {
                    "order": "desc",
                    "limit": "0x32",
                    "skip": null,
                    "return_count": true
                },
                "structure_type": "DoubleEntry"
            }
        ]
    }"#,
    );
    let r = &resp["result"];

    assert_eq!(r["next_cursor"], Value::Null);
    assert_eq!(r["count"], "0x1");
    let txs = &r["response"].as_array().unwrap();
    assert_eq!(txs.len(), 1);

    let tx = &r["response"][0]["value"];
    assert_eq!(
        tx["tx_hash"],
        "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
    );

    let records = &tx["records"].as_array().unwrap();
    assert_eq!(records.len(), 5);
    assert_eq!(records[0]["block_number"], "0x397f2e");
    for i in 1..4 {
        assert_eq!(records[i]["block_number"], "0x397f31");
    }
}
