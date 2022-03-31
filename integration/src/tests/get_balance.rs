use super::IntegrationTest;
use crate::const_definition::{GENESIS_BUILT_IN_ADDRESS_1, MERCURY_URI};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, AssetType, GetBalancePayload, JsonItem, Ownership};

use std::collections::HashSet;

fn test_get_balance() {
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(
        response.balances[0].ownership,
        Ownership::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string())
    );
    assert_eq!(response.balances[0].asset_info.asset_type, AssetType::CKB);
    println!("free: {:?}", response.balances[0].free);
    println!("occupied: {:?}", response.balances[0].occupied);
    println!("frozen: {:?}", response.balances[0].frozen);
    println!("claimable: {:?}", response.balances[0].claimable);
}

fn test_get_balance_udt() {}

inventory::submit!(IntegrationTest {
    name: "test_get_balance",
    test_fn: test_get_balance
});

inventory::submit!(IntegrationTest {
    name: "test_get_balance_udt",
    test_fn: test_get_balance_udt
});
