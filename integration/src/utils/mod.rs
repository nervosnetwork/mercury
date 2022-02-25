use anyhow::Result;
use core::panic;
use serde_json;

use std::env;
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

pub fn post_http_request(body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let mercury_testnet_host =
        env::var("MERCURY_TESTNET_HOST").unwrap_or_else(|_| String::from("http://127.0.0.1:8116"));
    let resp = client
        .post(mercury_testnet_host)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .unwrap();
    if !resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();
    println!("[request]:\n{}", body);
    println!("[response]:\n{}\n", text);

    serde_json::from_str(&text).unwrap()
}
