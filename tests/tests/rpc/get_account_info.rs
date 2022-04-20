use super::common::post_http_request;
use serde_json::Value;

#[test]
fn test_get_account_info_by_ckb_identity() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x00fa22aa0aaf155a6c816634c61512046b08923111"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x1");
    assert_eq!(r["account_type"], "Acp".to_string());
    assert_eq!(r["account_address"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn".to_string());
}

#[test]
fn test_get_account_info_by_pw_lock_identity() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Identity",
                "value": "0x01adabffb9c27cb4af100ce7bca6903315220e87a2"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x3");
    assert_eq!(r["account_type"], "PwLock".to_string());
    assert_eq!(r["account_address"], "ckt1qpvvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdxxqdd40lmnsnukjh3qr88hjnfqvc4yg8g0gskp8ffv".to_string());
}

#[test]
fn test_get_account_info_by_secp_address() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x1");
    assert_eq!(r["account_type"], "Acp".to_string());
    assert_eq!(r["account_address"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn".to_string());
}

#[test]
fn test_get_account_info_by_acp_address() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x1");
    assert_eq!(r["account_type"], "Acp".to_string());
    assert_eq!(r["account_address"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn".to_string());
}

#[test]
fn test_get_account_info_by_pw_lock_address() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qpvvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdxxqdd40lmnsnukjh3qr88hjnfqvc4yg8g0gskp8ffv"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x3");
    assert_eq!(r["account_type"], "PwLock".to_string());
    assert_eq!(r["account_address"], "ckt1qpvvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdxxqdd40lmnsnukjh3qr88hjnfqvc4yg8g0gskp8ffv".to_string());
}

#[test]
fn test_get_account_info_by_cheque_address() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qpsdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4kq29yywse6zu05ez3s64xmtdkl6074rac6zh7h2ln2w035d2lnh32ylk5ydmjq5ypwqs4asnr"
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    assert_ne!(resp["error"], Value::Null);
}

#[test]
fn test_get_account_info_by_out_point() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_account_info",
        "params": [{
            "item": {
                "type": "OutPoint",
                "value": {
                    "tx_hash": "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed",
                    "index": "0x0"
                }
            },
            "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
            }
        }],
        "id": 42
    }"#,
    );
    let r = &resp["result"];
    assert_eq!(r["account_number"], "0x1");
    assert_eq!(r["account_type"], "Acp".to_string());
    assert_eq!(r["account_address"], "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn".to_string());
}
