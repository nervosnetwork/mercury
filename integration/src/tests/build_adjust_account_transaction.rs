use super::IntegrationTest;
use crate::const_definition::{
    MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::{
    acp::build_acp_address, new_identity_from_secp_address, pw_lock::build_pw_lock_address,
};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AdjustAccountPayload, AssetInfo, GetBalancePayload, JsonItem, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_adjust_account",
    test_fn: test_adjust_account
});
fn test_adjust_account() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    let (address, address_pk, _) = prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let acp_address = build_acp_address(&address).unwrap();

    // acp number: 5
    prepare_account(udt_hash, &address, &address, &address_pk, Some(5)).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(acp_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(710_0000_0000u128, response.balances[0].occupied.into());

    // acp number: 1
    prepare_account(udt_hash, &address, &address, &address_pk, Some(1)).unwrap();

    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());

    // acp number: 0
    prepare_account(udt_hash, &address, &address, &address_pk, Some(0)).unwrap();

    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 0);
}

inventory::submit!(IntegrationTest {
    name: "test_adjust_account_pw_lock",
    test_fn: test_adjust_account_pw_lock
});
fn test_adjust_account_pw_lock() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    let (address, address_pk, _) = prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let pw_lock_address = build_pw_lock_address(&address_pk);

    // acp number: 5
    prepare_account(udt_hash, &pw_lock_address, &address, &address_pk, Some(5)).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(pw_lock_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(5_0000_0000u128, response.balances[0].free.into());
    assert_eq!(710_0000_0000u128, response.balances[0].occupied.into());

    // account number: 1
    prepare_account(udt_hash, &pw_lock_address, &address, &address_pk, Some(1)).unwrap();

    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert!(572_0000_0000u128 < response.balances[0].free.into());
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());

    // account number: 0
    prepare_account(udt_hash, &pw_lock_address, &address, &address_pk, Some(0)).unwrap();

    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert!(714_0000_0000u128 < response.balances[0].free.into());
    assert_eq!(0u128, response.balances[0].occupied.into());
}

inventory::submit!(IntegrationTest {
    name: "test_adjust_account_from_multi",
    test_fn: test_adjust_account_from_multi
});
fn test_adjust_account_from_multi() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let udt_holder_address = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let udt_holder_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    let (address, address_pk, out_point) =
        prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let acp_address = build_acp_address(&address).unwrap();

    // acp number: 5
    let adjust_account_payload = AdjustAccountPayload {
        item: JsonItem::Address(acp_address.to_string()),
        from: vec![
            JsonItem::OutPoint(out_point.clone()),
            JsonItem::Address(address.to_string()),
        ],
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        account_number: Some(5u32.into()),
        extra_ckb: None,
        fee_rate: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_adjust_account_transaction(adjust_account_payload)
        .unwrap()
        .unwrap();
    let tx = sign_transaction(tx, &[address_pk.to_owned()]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(acp_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(710_0000_0000u128, response.balances[0].occupied.into());

    // account number: 1
    let adjust_account_payload = AdjustAccountPayload {
        item: JsonItem::Address(acp_address.to_string()),
        from: vec![
            JsonItem::OutPoint(out_point.clone()),
            JsonItem::Address(address.to_string()),
        ],
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        account_number: Some(1u32.into()),
        extra_ckb: None,
        fee_rate: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_adjust_account_transaction(adjust_account_payload)
        .unwrap()
        .unwrap();
    let tx = sign_transaction(tx, &[address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());

    // transfer udt
    let from_identity = new_identity_from_secp_address(&udt_holder_address.to_string()).unwrap();
    let transfer_payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client
        .build_transfer_transaction(transfer_payload)
        .unwrap();
    let tx = sign_transaction(tx, &[udt_holder_address_pk.to_owned()]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // account number: 0
    let adjust_account_payload = AdjustAccountPayload {
        item: JsonItem::Address(acp_address.to_string()),
        from: vec![
            JsonItem::OutPoint(out_point),
            JsonItem::Address(address.to_string()),
        ],
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        account_number: Some(0u32.into()),
        extra_ckb: None,
        fee_rate: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_adjust_account_transaction(adjust_account_payload);

    assert!(tx.is_err());
}
