use anyhow::Result;
use core::panic;
use serde_json;

use std::ffi::OsStr;
use std::process::{Child, Command};

pub fn run<I, S>(bin: &str, args: I) -> Result<Child>
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<OsStr>,
{
    let child = Command::new(bin.to_owned())
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .spawn()
        .expect("run command");
    Ok(child)
}

pub fn post_http_request(uri: String, body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(uri)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .unwrap();
    if !resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();

    // println!("[request]:\n{}", body);
    // println!("[response]:\n{}\n", text);

    serde_json::from_str(&text).unwrap()
}
