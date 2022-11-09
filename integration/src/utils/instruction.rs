use crate::const_definition::{
    ANYONE_CAN_PAY_DEVNET_TYPE_HASH, CELL_BASE_MATURE_EPOCH, CHEQUE_DEVNET_TYPE_HASH, CKB_URI,
    DAO_DEVNET_TYPE_HASH, GENESIS_BUILT_IN_ADDRESS_1, GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY,
    GENESIS_EPOCH_LENGTH, MERCURY_URI, PW_LOCK_DEVNET_TYPE_HASH, RPC_TRY_COUNT,
    RPC_TRY_INTERVAL_SECS, SIGHASH_TYPE_HASH, SUDT_DEVNET_TYPE_HASH, UDT_1_HASH,
    UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::secp::prepare_secp_address_with_ckb_capacity;
use crate::utils::address::{
    acp::build_acp_address, cheque::build_cheque_address, get_udt_hash_by_owner,
};
use crate::utils::rpc_client::{CkbRpcClient, MercuryRpcClient};
use crate::utils::signer::sign_transaction;

use anyhow::Result;
use ckb_jsonrpc_types::{OutPoint, OutputsValidator, Transaction};
use ckb_types::H256;
use common::lazy::{
    ACP_CODE_HASH, CHEQUE_CODE_HASH, DAO_CODE_HASH, PW_LOCK_CODE_HASH, SECP256K1_CODE_HASH,
    SUDT_CODE_HASH,
};
use common::Address;
use core_rpc_types::{
    AdjustAccountPayload, AssetInfo, AssetType, IOType, JsonItem, OutputCapacityProvider,
    SudtIssuePayload, ToInfo, TransferPayload,
};
use serde::Serialize;

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
    let child = Command::new(bin)
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
    );
    let mercury = if let Ok(mercury) = mercury {
        mercury
    } else {
        teardown(vec![ckb]);
        panic!("start mercury");
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = mercury_client.get_mercury_info();
        if resp.is_ok() {
            let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
            mercury_client.wait_sync();

            // This step is used to make mercury enter the normal serial sync loop state
            // only then can all initialization be completed
            if generate_blocks(1).is_err() {
                teardown(vec![ckb, mercury]);
                panic!("generate block when start mercury");
            }

            // init built-in script code hash
            let _ = SECP256K1_CODE_HASH.set(SIGHASH_TYPE_HASH);
            let _ = SUDT_CODE_HASH.set(SUDT_DEVNET_TYPE_HASH);
            let _ = ACP_CODE_HASH.set(ANYONE_CAN_PAY_DEVNET_TYPE_HASH);
            let _ = CHEQUE_CODE_HASH.set(CHEQUE_DEVNET_TYPE_HASH);
            let _ = DAO_CODE_HASH.set(DAO_DEVNET_TYPE_HASH);
            let _ = PW_LOCK_CODE_HASH.set(PW_LOCK_DEVNET_TYPE_HASH);

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
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number) * GENESIS_EPOCH_LENGTH {
            let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
            let block_hash = ckb_client.generate_block().expect("generate block");
            println!("generate new block: {:?}", block_hash.to_string());
        }
    }
}

pub(crate) fn issue_udt_1() -> Result<()> {
    if UDT_1_HASH.get().is_some() {
        return Ok(());
    }

    // issue udt
    let (owner_address, owner_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(500_0000_0000)?;
    let udt_hash = get_udt_hash_by_owner(&owner_address)?;
    let _tx_hash = issue_udt_with_acp(&owner_address, &owner_address_pk, 20_000_000_000u128)?;
    let acp_address = build_acp_address(&owner_address)?;

    UDT_1_HASH.set(udt_hash).expect("init UDT_HASH_1");
    UDT_1_HOLDER_ACP_ADDRESS
        .set(acp_address)
        .expect("init UDT_1_HOLDER_ACP_ADDRESS");
    UDT_1_HOLDER_ACP_ADDRESS_PK
        .set(owner_address_pk)
        .expect("init UDT_1_HOLDER_ACP_ADDRESS_PK");
    Ok(())
}

pub(crate) fn fast_forward_epochs(number: usize) -> Result<()> {
    generate_blocks(GENESIS_EPOCH_LENGTH as usize * number + 1)
}

pub(crate) fn generate_blocks(number: usize) -> Result<()> {
    let ckb_rpc_client = CkbRpcClient::new(CKB_URI.to_string());
    let mut block_hash: H256 = H256::default();
    for _ in 0..number {
        block_hash = ckb_rpc_client.generate_block()?;
    }
    let mercury_rpc_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    mercury_rpc_client.wait_block(block_hash);
    Ok(())
}

pub(crate) fn aggregate_transactions_into_blocks() -> Result<()> {
    generate_blocks(3)
}

pub(crate) fn send_transaction_to_ckb(tx: Transaction) -> Result<H256> {
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let tx_hash = ckb_client.send_transaction(tx, OutputsValidator::Passthrough)?;
    println!("send tx: 0x{}", tx_hash);
    aggregate_transactions_into_blocks()?;
    Ok(tx_hash)
}

pub(crate) fn issue_udt_with_cheque(
    owner_address: &Address,
    owner_pk: &H256,
    to_address: &Address,
    udt_amount: u128,
) -> Result<H256> {
    let cheque_address =
        build_cheque_address(to_address, owner_address).expect("build cheque address");
    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        from: vec![JsonItem::Address(owner_address.to_string())],
        to: vec![ToInfo {
            address: cheque_address.to_string(),
            amount: udt_amount.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_sudt_issue_transaction(payload)?;
    let tx = sign_transaction(tx, &[owner_pk.to_owned()])?;

    // send tx to ckb node
    send_transaction_to_ckb(tx)
}

pub(crate) fn issue_udt_with_acp(
    owner_address: &Address,
    owner_pk: &H256,
    udt_amount: u128,
) -> Result<H256> {
    let acp_address = build_acp_address(owner_address).expect("build acp address");
    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        from: vec![JsonItem::Address(owner_address.to_string())],
        to: vec![ToInfo {
            address: acp_address.to_string(),
            amount: udt_amount.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_sudt_issue_transaction(payload)?;
    let tx = sign_transaction(tx, &[owner_pk.to_owned()])?;

    // send tx to ckb node
    send_transaction_to_ckb(tx)
}

pub(crate) fn prepare_account(
    udt_hash: &H256,
    item: &Address,
    from_address: &Address,
    from_address_pk: &H256,
    account_number: Option<u32>,
) -> Result<()> {
    let payload = AdjustAccountPayload {
        item: JsonItem::Address(item.to_string()),
        from: vec![JsonItem::Address(from_address.to_string())],
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        account_number: account_number.map(Into::into),
        extra_ckb: None,
        fee_rate: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_adjust_account_transaction(payload)?;
    if let Some(tx) = tx {
        let tx = sign_transaction(tx, &[from_address_pk.to_owned()])?;
        let _tx_hash = send_transaction_to_ckb(tx)?;
    }
    Ok(())
}

pub fn dump_data<T>(data: &T, file_name: &str) -> Result<()>
where
    T: ?Sized + Serialize,
{
    let json_string = serde_json::to_string_pretty(data)?;
    std::fs::write(file_name, json_string).map_err(Into::into)
}

pub(crate) fn prepare_ckb_capacity(address: &Address, capacity: u64) -> Result<OutPoint> {
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string())],
        to: vec![ToInfo {
            address: address.to_string(),
            amount: (capacity as u128).into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload)?;
    let tx = sign_transaction(tx, &[GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY])?;

    // send tx to ckb node
    let tx_hash = send_transaction_to_ckb(tx)?;

    let tx_info = mercury_client
        .get_transaction_info(tx_hash)?
        .transaction
        .expect("get transaction info");
    let out_point = &tx_info
        .records
        .into_iter()
        .find(|record| {
            record.asset_info.asset_type == AssetType::CKB
                && record.amount == (capacity as u128).into()
                && record.io_type == IOType::Output
        })
        .expect("find record")
        .out_point;

    Ok(out_point.to_owned())
}
