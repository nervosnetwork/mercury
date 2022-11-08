use super::super::IntegrationTest;
use crate::const_definition::{MERCURY_URI, UDT_1_HASH};
use crate::utils::address::acp::build_acp_address;
use crate::utils::address::generate_rand_secp_address_pk_pair;
use crate::utils::address::omni_lock::{
    build_omni_acp_account_address, generate_omni_secp_address_pk_pair,
    prepare_omni_ethereum_address_with_capacity, prepare_omni_secp_address_with_capacity,
};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_ckb_capacity, prepare_secp_address_with_ckb_capacity,
    send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;
use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, OutputCapacityProvider, ToInfo,
    TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_omni_secp_transfer_ckb",
    test_fn: test_omni_secp_transfer_ckb
});
fn test_omni_secp_transfer_ckb() {
    let (identity, address, address_pk, _out_point) =
        prepare_omni_secp_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Identity(identity.encode()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 0u128.into());
    assert_eq!(balance.balances[0].free, 300_0000_0000u128.into());
    assert_eq!(balance.balances[0].frozen, 0u128.into());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(address.to_string())],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    // dump_data(&tx, "tx.json").unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_secp_transfer_ckb_identity",
    test_fn: test_omni_secp_transfer_ckb_identity
});
fn test_omni_secp_transfer_ckb_identity() {
    let (identity, _address, address_pk, _out_point) =
        prepare_omni_secp_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let pks = vec![address_pk];
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Identity(hex::encode(identity.0))],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    // dump_data(&tx, "tx.json").unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_eth_transfer_ckb",
    test_fn: test_omni_eth_transfer_ckb
});
fn test_omni_eth_transfer_ckb() {
    let (identity, address, address_pk, _out_point) =
        prepare_omni_ethereum_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Identity(identity.encode()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 0u128.into());
    assert_eq!(balance.balances[0].free, 300_0000_0000u128.into());
    assert_eq!(balance.balances[0].frozen, 0u128.into());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(address.to_string())],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Identity(identity.encode()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 0u128.into());
    assert!(199_0000_0000u128 < balance.balances[0].free.into());
    assert_eq!(balance.balances[0].frozen, 0u128.into());

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_to_omni_acp",
    test_fn: test_transfer_ckb_to_omni_acp
});
fn test_transfer_ckb_to_omni_acp() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    let (secp_address, secp_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let (omni_identity, omni_address, _omni_address_pk) = generate_omni_secp_address_pk_pair();
    let omni_account_address = build_omni_acp_account_address(&omni_address).unwrap();

    // acp number: 1
    prepare_account(
        udt_hash,
        &omni_address,
        &secp_address,
        &secp_address_pk,
        Some(1),
    )
    .unwrap();

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(secp_address.to_string())],
        to: vec![ToInfo {
            address: omni_account_address.to_string(),
            amount: 1_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::To),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[secp_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(omni_identity.0)),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(2_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(0u128, response.balances[1].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_transfer_ckb_from_acp_to_acp",
    test_fn: test_omni_transfer_ckb_from_acp_to_acp
});
fn test_omni_transfer_ckb_from_acp_to_acp() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    let (secp_address, secp_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let acp_address = build_acp_address(&secp_address).unwrap();

    let (omni_identity, omni_address, omni_address_pk) = generate_omni_secp_address_pk_pair();
    let omni_account_address = build_omni_acp_account_address(&omni_address).unwrap();

    prepare_account(
        udt_hash,
        &omni_address,
        &secp_address,
        &secp_address_pk,
        Some(1),
    )
    .unwrap();

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(omni_account_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(1_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());

    prepare_account(
        udt_hash,
        &secp_address,
        &secp_address,
        &secp_address_pk,
        Some(1),
    )
    .unwrap();

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(omni_account_address.to_string())],
        to: vec![ToInfo {
            address: acp_address.to_string(),
            amount: 1000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::To),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[omni_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(omni_identity.0)),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert!(9999_0000u128 < response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(0u128, response.balances[1].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_transfer_ckb_change",
    test_fn: test_omni_transfer_ckb_change
});
fn test_omni_transfer_ckb_change() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    let (secp_address, secp_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let _acp_address = build_acp_address(&secp_address).unwrap();

    let (omni_identity, omni_address, omni_address_pk) = generate_omni_secp_address_pk_pair();
    let _omni_account_address = build_omni_acp_account_address(&omni_address).unwrap();
    let _out_point = prepare_ckb_capacity(&omni_address, 125_0000_0000).unwrap();

    prepare_account(
        udt_hash,
        &omni_address,
        &secp_address,
        &secp_address_pk,
        Some(1),
    )
    .unwrap();

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(omni_identity.0)),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(1_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(125_0000_0000u128, response.balances[1].free.into());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Identity(hex::encode(omni_identity.0))],
        to: vec![ToInfo {
            address: secp_address.to_string(),
            amount: 61_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client
        .build_transfer_transaction(payload.clone())
        .unwrap();
    let tx = sign_transaction(tx, &[omni_address_pk.clone()]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let response = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(1_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
    assert!(63_0000_0000u128 < response.balances[1].free.into());

    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[omni_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let response = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert!(9999_0000u128 < response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
}
