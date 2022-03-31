use crate::const_definition::{
    CELL_BASE_MATURE_EPOCH, CKB_URI, GENESIS_BUILT_IN_ADDRESS_1,
    GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY, MERCURY_URI, RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS,
};
use crate::utils::address::generate_rand_secp_address_pk_pair;
use crate::utils::rpc_client::{CkbRpcClient, MercuryRpcClient};
use crate::utils::signer::Signer;
use core_rpc_types::{AssetInfo, From, JsonItem, Mode, Source, To, ToInfo, TransferPayload};

use anyhow::Result;
use ckb_jsonrpc_types::OutputsValidator;
use ckb_types::H256;
use common::Address;

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
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = ckb_client.local_node_info();
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
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = mercury_client.get_mercury_info();
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
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let epoch_view = ckb_client.get_current_epoch().expect("get_current_epoch");
    let current_epoch_number = epoch_view.number.value();
    if current_epoch_number < CELL_BASE_MATURE_EPOCH {
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number) * 1000 {
            let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
            let block_hash = ckb_client.generate_block().expect("generate block");
            println!("generate new block: {:?}", block_hash.to_string());
        }
    }
}

pub(crate) fn generate_blocks() -> Result<()> {
    let ckb_rpc_client = CkbRpcClient::new(CKB_URI.to_string());
    let mut block_hash: H256 = H256::default();
    for _ in 0..3 {
        block_hash = ckb_rpc_client.generate_block().expect("generate block");
    }
    let mercury_rpc_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    mercury_rpc_client.wait_block(block_hash)
}

pub(crate) fn prepare_address_with_ckb_capacity(capacity: u64) -> Result<(Address, H256)> {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string())],
            source: Source::Free,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: address.to_string(),
                amount: capacity.to_string(),
            }],
            mode: Mode::HoldByFrom,
        },
        pay_fee: None,
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload)?;
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY)?;

    // send tx to ckb node
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let tx_hash = ckb_client.send_transaction(tx, OutputsValidator::Passthrough)?;
    println!("send tx: 0x{}", tx_hash.to_string());
    generate_blocks()?;

    Ok((address, pk))
}
