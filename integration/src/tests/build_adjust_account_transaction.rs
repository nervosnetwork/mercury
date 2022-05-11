use super::IntegrationTest;
use crate::const_definition::{MERCURY_URI, UDT_1_HASH};
use crate::utils::address::{build_acp_address, build_pw_lock_address};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_secp_address_with_ckb_capacity,
};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, GetBalancePayload, JsonItem};

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
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(710_0000_0000u128, response.balances[0].occupied.into());

    // account number: 1
    prepare_account(udt_hash, &pw_lock_address, &address, &address_pk, Some(1)).unwrap();

    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());

    // account number: 0
    let ret = prepare_account(udt_hash, &pw_lock_address, &address, &address_pk, Some(0));
    assert!(ret.is_err());
}
