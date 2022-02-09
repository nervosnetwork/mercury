use super::common::post_http_request;

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
    assert_eq!(r["account_number"], 1);
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
    assert_eq!(r["account_number"], 1);
}
