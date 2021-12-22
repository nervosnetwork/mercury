use core::panic;
use serde_json::Value;
use std::{i64, slice::Iter};

pub fn post_http_request(body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let resp = client.post("http://127.0.0.1:8116")
        .header("content-type", "application/json")
        .body(body)
        .send().unwrap();
    if ! resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();
    println!("[request]:\n{}", body);
    println!("[response]:\n{}\n", text);

    serde_json::from_str(&text).unwrap()
}

pub fn check_amount(outputs: Iter<Value>, input_total: i64, fee: Option<i64>) {
    let mut output_total: i64 = 0;
    for output in outputs {
        let hex_str_amount = output["capacity"].as_str().unwrap().trim_start_matches("0x");
        output_total += i64::from_str_radix(hex_str_amount, 16).unwrap();
    }
    if let Some(fee) = fee {
        assert_eq!(fee + output_total, input_total);
    } else {
        assert!(output_total + 100000 > input_total);
    }

}
