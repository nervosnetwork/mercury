use serde_json::Value;

use super::common::post_http_request;

#[test]
fn test_structure_type_native() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
            "tx_hash": "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112",
            "index": "0x2"
            },
            "structure_type": "Native"
        }]
    }"#);
    let r = &resp["result"];
    assert_eq!(r["value"]["tx_status"]["status"].as_str().unwrap(), "committed");
    assert_eq!(r["value"]["tx_status"]["block_hash"].as_str().unwrap(), "0x54af022e0fdebed3690ee6d6e2368f3c976c6ad6cfcdbfa307869fd57c2d5129");

    let tx = &r["value"]["transaction"];
    assert_eq!(tx["hash"].as_str().unwrap(), "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1");

    let inputs = &tx["inputs"].as_array().unwrap();
    assert!(inputs.len() == 2);
    assert_eq!(inputs[1]["previous_output"]["tx_hash"].as_str().unwrap(), "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112");
    assert_eq!(inputs[1]["previous_output"]["index"].as_str().unwrap(), "0x2");

    let outputs = &tx["outputs"].as_array().unwrap();
    assert!(outputs.len() == 3);
    let mut output = &Value::Null;
    for _output in outputs.iter() {
        if "0x7efc5e6418".eq(_output["capacity"].as_str().unwrap()) {
            output = _output;
            break;
        }
    }
    assert!(output != &Value::Null);
    assert_eq!(output["lock"]["args"].as_str().unwrap(), "0xa3b8598e1d53e6c5e89e8acb6b4c34d3adb13f2b");
    assert!(output["type"] == Value::Null);

    let outputs_data = &tx["outputs_data"].as_array().unwrap();
    assert!(outputs_data.len() == 3);
}


#[test]
fn test_structure_type_double_entry() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
            "tx_hash": "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112",
            "index": "0x2"
            },
            "structure_type": "DoubleEntry"
        }]
    }"#);
    let r = &resp["result"];
    assert_eq!(r["type"].as_str().unwrap(), "TransactionInfo");

    let tx = &r["value"];
    assert_eq!(r["value"]["tx_hash"].as_str().unwrap(), "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1");

    let records = tx["records"].as_array().unwrap();
    assert!(records.len() == 8);

    // input #1
    let mut record = &Value::Null;
    for _record in records.iter() {
        println!("{}", _record);
        if "-561599924694".eq(_record["amount"].as_str().unwrap()) {
            record = _record;
            break;
        }
    }
    assert!(record != &Value::Null);
    assert_eq!(record["ownership"]["value"].as_str().unwrap(), "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdrhpvcu82numz73852ed45cdxn4kcn72cr4338a");

    // output #0
    let mut record = &Value::Null;
    for _record in records.iter() {
        println!("{}", _record);
        if "100".eq(_record["amount"].as_str().unwrap()) {
            record = _record;
            break;
        }
    }
    assert!(record != &Value::Null);
    assert_eq!(record["ownership"]["value"].as_str().unwrap(), "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqv8c706nuqkxchp0sw996qqunhmkkgesqs0rppmc");


    // output #2
    let mut record = &Value::Null;
    for _record in records.iter() {
        println!("{}", _record);
        if "545399923736".eq(_record["amount"].as_str().unwrap()) {
            record = _record;
            break;
        }
    }
    assert!(record != &Value::Null);
    assert_eq!(record["ownership"]["value"].as_str().unwrap(), "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdrhpvcu82numz73852ed45cdxn4kcn72cr4338a");
}


#[test]
fn test_structure_type_native_unspent_outpoint() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
                "index": "0x1"
            },
            "structure_type": "Native"
        }]
    }"#);
    assert!(resp["error"] != Value::Null);
}


#[test]
fn test_structure_type_native_nonexistent_outpoint() {
    // The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
    let resp = post_http_request(r#"{
        "id": 42,
        "jsonrpc": "2.0",
        "method": "get_spent_transaction",
        "params": [{
            "outpoint": {
                "tx_hash": "0x1f8c79eb3671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
                "index": "0x1"
            },
            "structure_type": "Native"
        }]
    }"#);
    assert!(resp["error"] != Value::Null);
}
