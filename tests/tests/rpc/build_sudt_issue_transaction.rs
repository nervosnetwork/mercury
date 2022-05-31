use super::common::check_amount;
use super::common::post_http_request;
use serde_json::Value;

#[test]
fn test_address() {
    let resp = post_http_request(
        r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "build_sudt_issue_transaction",
        "params": [
            {
                "owner": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9",
                "from": [
                    {
                        "type": "Address",
                        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
                    }
                ],
                "to": [
                    {
                        "address": "ckt1qqdpunl0xn6es2gx7azmqj870vggjer7sg6xqa8q7vkzan3xea43uqt6g2dxvxxjtdhfvfs0f67gwzgrcrfg3gj9yywse6zu05ez3s64xmtdkl6074rac6q3f7cvk",
                        "amount": "0x5f5e100"
                    }
                ],
                "output_capacity_provider": "From",
                "fee_rate": "0x3e8",
                "since": null
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

    let sudt_output = outputs
        .iter()
        .find(|output| output["type"] != Value::Null)
        .unwrap();
    assert_eq!(
        sudt_output["type"]["code_hash"],
        "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4"
    );
    check_amount(outputs.iter(), 1000000000000, None);
}
