use super::IntegrationTest;
use crate::utils::post_http_request;

fn test_get_balance() {
    let resp = post_http_request(
        r#"{
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": [{
            "item": {
                "type": "Address",
                "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqwgx292hnvmn68xf779vmzrshpmm6epn4c0cgwga"
            },
            "asset_infos": [{
                "asset_type": "CKB",
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }]
        }],
        "id": 100
    }"#,
    );
    let r = &resp["result"];

    let balances = &r["balances"].as_array().unwrap();
    assert_eq!(balances.len(), 1);
    let balance = &balances[0];
    assert_eq!(balance["ownership"]["type"], "Address");
    assert_eq!(balance["ownership"]["value"], "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqwgx292hnvmn68xf779vmzrshpmm6epn4c0cgwga");
    assert_eq!(balance["asset_info"]["asset_type"], "CKB");
    assert_eq!(
        balance["asset_info"]["udt_hash"],
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(balance["freezed"], "2000000000000000000"); // todo: check free or freezed
}

fn test_get_balance_udt() {
    println!("Running get_balance_udt test")
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance",
    test_fn: test_get_balance
});

inventory::submit!(IntegrationTest {
    name: "test_get_balance_udt",
    test_fn: test_get_balance_udt
});
