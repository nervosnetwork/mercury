use super::super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::omni_lock::{
    prepare_omni_ethereum_address_with_capacity, prepare_omni_secp_address_with_capacity,
};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, AssetType, GetBalancePayload, JsonItem};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_get_balance_by_omni_ethereum",
    test_fn: test_get_balance_by_omni_ethereum
});
fn test_get_balance_by_omni_ethereum() {
    let (identity, address, address_pk, out_point) =
        prepare_omni_ethereum_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let _pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

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

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point),
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

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
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
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance_by_omni_secp",
    test_fn: test_get_balance_by_omni_secp
});
fn test_get_balance_by_omni_secp() {
    let (identity, address, address_pk, out_point) =
        prepare_omni_secp_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let _pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

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

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point),
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

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
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
}
