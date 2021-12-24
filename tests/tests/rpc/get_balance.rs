use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_address_ckb() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }],
            "tip_block_number": 3636218
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3636218);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["ownership"]["type"], "Address");
    assert_eq!(balance["ownership"]["value"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqvrnuvqd6zmgrqn60rnsesy23mvex5vy9q0g8hfd");
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(balance["asset_info"]["udt_hash"], "0x0000000000000000000000000000000000000000000000000000000000000000");
    assert_eq!(balance["free"], "6053944763147");
}

#[test]
fn test_address_udt() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
            },
            "asset_infos": [{
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": 3767610
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3767610);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["ownership"]["type"], "Address");
    assert_eq!(balance["ownership"]["value"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn");
    assert_eq!(balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(balance["asset_info"]["udt_hash"], "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd");
    assert_eq!(balance["free"], "60");
    assert_eq!(balance["occupied"], "0");
    assert_eq!(balance["freezed"], "0");
    assert_eq!(balance["claimable"], "0");
}

#[test]
fn test_address_all() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
            },
            "asset_infos": [],
            "tip_block_number": 3767610
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3767610);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let (udt_balance, ckb_balance) = if balances[0]["asset_info"]["asset_type"] == "UDT" {
        (&balances[0], &balances[1])
    } else {
        (&balances[1], &balances[0])
    };

    assert_eq!(udt_balance["ownership"]["type"], "Address");
    assert_eq!(udt_balance["ownership"]["value"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn");
    assert_eq!(udt_balance["asset_info"]["asset_type"], "UDT");
    assert_eq!(udt_balance["asset_info"]["udt_hash"], "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd");
    assert_eq!(udt_balance["free"], "60");
    assert_eq!(udt_balance["occupied"], "0");
    assert_eq!(udt_balance["freezed"], "0");
    assert_eq!(udt_balance["claimable"], "0");

    assert_eq!(ckb_balance["ownership"]["type"], "Address");
    assert_eq!(ckb_balance["ownership"]["value"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn");
    assert_eq!(ckb_balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(ckb_balance["asset_info"]["udt_hash"], "0x0000000000000000000000000000000000000000000000000000000000000000");
}


#[test]
fn test_identity_ckb() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }],
            "tip_block_number": 3769130
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3769130);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 2);

    let acp_balance = balances.iter().find(|balance|
        balance["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn")
        .unwrap();
    assert_eq!(acp_balance["ownership"]["type"], "Address");
    assert_eq!(acp_balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(acp_balance["free"], "1979699999470");


    let secp_balance = balances.iter().find(|balance|
        balance["ownership"]["value"] == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9")
        .unwrap();
    assert_eq!(secp_balance["ownership"]["type"], "Address");
    assert_eq!(secp_balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(secp_balance["free"], "1000000000000");
}

#[test]
fn test_identity_udt() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": 3769130
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3769130);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);

    assert_eq!(balances[0]["ownership"]["type"], "Address");
    assert_eq!(balances[0]["ownership"]["value"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn");
    assert_eq!(balances[0]["asset_info"]["asset_type"], "UDT");
    assert_eq!(balances[0]["asset_info"]["udt_hash"], "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd");
}

#[test]
fn test_identity_all() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [],
            "tip_block_number": 3769130
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3769130);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 3);

    let acp_ckb_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "CKB"
        && balance["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn")
        .unwrap();
    assert_eq!(acp_ckb_balance["free"], "1979699999470");

    let secp_ckb_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "CKB"
        && balance["ownership"]["value"] == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9")
        .unwrap();
    assert_eq!(secp_ckb_balance["free"], "1000000000000");


    let acp_udt_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "UDT"
        && balance["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn")
        .unwrap();
    assert_eq!(acp_udt_balance["free"], "60");
}


#[test]
fn test_identity_multiple_assets() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
            "type": "Identity",
            "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
            }],
            "tip_block_number": 3769130
        }],
        "id": 100
    }"#);
    let r = &resp["result"];
    assert_eq!(r["tip_block_number"], 3769130);

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 3);

    let acp_ckb_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "CKB"
        && balance["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn")
        .unwrap();
    assert_eq!(acp_ckb_balance["free"], "1979699999470");

    let secp_ckb_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "CKB"
        && balance["ownership"]["value"] == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9")
        .unwrap();
    assert_eq!(secp_ckb_balance["free"], "1000000000000");

    let acp_udt_balance = balances.iter().find(|balance|
        balance["asset_info"]["asset_type"] == "UDT"
        && balance["ownership"]["value"] == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn")
        .unwrap();
    assert_eq!(acp_udt_balance["free"], "60");
}

#[test]
fn test_record() {
    // live cell of the given record: output #0 in tx 0xea0b230104fd3be2cc33ab50c3d591dc6cefbe8ed83f7e63c8142de4b5a0ee72
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Record",
                "value": "ea0b230104fd3be2cc33ab50c3d591dc6cefbe8ed83f7e63c8142de4b5a0ee720000000000636b7431717a646130637230386d38356863386a6c6e6670337a65723778756c656a79777434396b7432727230767468797761613530787773717736766a7a79396b616878336c79766c67617038647038657764386738307063676365787a726a"
            },
            "asset_infos": []
        }],
        "id": 10
    }"#);
    let r = &resp["result"];

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["ownership"]["type"], "Address");
    assert_eq!(balance["ownership"]["value"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqw6vjzy9kahx3lyvlgap8dp8ewd8g80pcgcexzrj");
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(balance["free"], "194703317445");
}


#[test]
fn test_illegal_address() {
    let resp = post_http_request(r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1q"
            },
            "asset_infos": [],
            "tip_block_number": 3636218
        }],
        "id": 100
    }"#);
    assert_ne!(resp["error"], Value::Null);
}
