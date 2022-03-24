use crate::const_definition::{
    CELL_BASE_MATURE_EPOCH, CKB_URI, MERCURY_URI, RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS,
};
use crate::utils::rpc_client::{
    post_http_request, try_post_http_request, CkbRpcClient, MercuryRpcClient,
};

use anyhow::Result;

use std::ffi::OsStr;
use std::panic;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

pub(crate) fn run_command<I, S>(bin: &str, args: I) -> Result<Child>
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

pub(crate) fn setup() -> Vec<Child> {
    println!("Setup test environment...");
    let ckb = start_ckb_node();
    let (ckb, mercury) = start_mercury(ckb);
    vec![ckb, mercury]
}

pub(crate) fn teardown(childs: Vec<Child>) {
    for mut child in childs {
        child.kill().expect("teardown failed");
    }
}

pub(crate) fn start_ckb_node() -> Child {
    let ckb = run_command(
        "ckb",
        vec!["run", "-C", "dev_chain/dev", "--skip-spec-check"],
    )
    .expect("start ckb dev chain");
    for _try in 0..=RPC_TRY_COUNT {
        let resp = try_post_http_request(
            CKB_URI,
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "local_node_info"
            }"#,
        );
        if resp.is_ok() {
            unlock_frozen_capacity_in_genesis();
            return ckb;
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb]);
    panic!("Setup test environment failed");
}

pub(crate) fn start_mercury(ckb: Child) -> (Child, Child) {
    let mercury = run_command(
        "cargo",
        vec![
            "run",
            "--manifest-path",
            "../Cargo.toml",
            "--",
            "-c",
            "dev_chain/devnet_config.toml",
            "run",
        ],
    )
    .expect("start ckb dev chain");
    for _try in 0..=RPC_TRY_COUNT {
        let resp = try_post_http_request(
            MERCURY_URI,
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "get_mercury_info"
            }"#,
        );
        if resp.is_ok() {
            let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
            mercury_client.wait_sync();
            return (ckb, mercury);
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb, mercury]);
    panic!("Setup test environment failed");
}

fn unlock_frozen_capacity_in_genesis() {
    let resp = post_http_request(
        CKB_URI,
        r#"{
            "id": 42,
            "jsonrpc": "2.0",
            "method": "get_current_epoch"
        }"#,
    );
    let current_epoch_number = &resp["result"]["number"]
        .as_str()
        .expect("get current epoch number")
        .trim_start_matches("0x");
    let current_epoch_number = u64::from_str_radix(current_epoch_number, 16).unwrap();
    if current_epoch_number < CELL_BASE_MATURE_EPOCH {
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number + 1) * 1000 {
            let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
            let block_hash = ckb_client.generate_block().expect("generate block");
            println!("generate new block: {:?}", block_hash.to_string());
        }
    }
}

pub(crate) fn generate_block() -> Result<()> {
    let ckb_rpc_client = CkbRpcClient::new(CKB_URI.to_string());
    let block_hash = ckb_rpc_client.generate_block().expect("generate block");

    let mercury_rpc_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    mercury_rpc_client.wait_block(block_hash)
}
