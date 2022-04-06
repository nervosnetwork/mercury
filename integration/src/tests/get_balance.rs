use super::IntegrationTest;
use crate::const_definition::{CHEQUE_LOCK_EPOCH, GENESIS_BUILT_IN_ADDRESS_1, MERCURY_URI};
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    fast_forward_epochs, issue_udt_with_cheque, prepare_address_with_ckb_capacity,
};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, AssetType, GetBalancePayload, JsonItem, Ownership};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_get_balance_of_genesis_built_in_address_1",
    test_fn: test_get_balance_of_genesis_built_in_address_1
});
fn test_get_balance_of_genesis_built_in_address_1() {
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
    println!("GENESIS_BUILT_IN_ADDRESS_1:");
    println!("  free: {:?}", response.balances[0].free);
    println!("  occupied: {:?}", response.balances[0].occupied);
    println!("  frozen: {:?}", response.balances[0].frozen);
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance_of_identity_has_cheque",
    test_fn: test_get_balance_of_identity_has_cheque
});
fn test_get_balance_of_identity_has_cheque() {
    // prepare
    let (owner_address, owner_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // issue udt
    let _tx_hash = issue_udt_with_cheque(&owner_address, &owner_pk, &to_address, 100u64);

    let udt_hash = get_udt_hash_by_owner(&owner_address).unwrap();
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let owner_identity = new_identity_from_secp_address(&owner_address.to_string()).unwrap();
    let to_identity = new_identity_from_secp_address(&to_address.to_string()).unwrap();

    // get balance of to identity, AssetType::UDT
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash));
    let payload_to = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload_to.clone()).unwrap();
    let udt_balance = &to_balance.balances[0];
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(udt_balance.free, 100u64.to_string());

    // get balance of to identity, AssetType::CKB
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 0);

    // get balance of to identity, HashSet::new()
    let payload = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u64.to_string());

    // get balance of owner identity
    let payload_owner = GetBalancePayload {
        item: JsonItem::Identity(owner_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let owner_balance = mercury_client.get_balance(payload_owner.clone()).unwrap();
    let owner_left_capacity = owner_balance.balances[0].free.parse::<u64>().unwrap();

    assert_eq!(owner_balance.balances.len(), 1);
    assert!(owner_left_capacity < 88_0000_0000);
    assert!(owner_left_capacity > 87_0000_0000);

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // get balance of owner identity
    let owner_balance = mercury_client.get_balance(payload_owner).unwrap();
    let (ckb_balance, udt_balance) =
        if owner_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&owner_balance.balances[0], &owner_balance.balances[1])
        } else {
            (&owner_balance.balances[1], &owner_balance.balances[0])
        };

    assert_eq!(owner_balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 162_0000_0000u64.to_string());
    assert_eq!(udt_balance.free, 100u64.to_string());

    // get balance of to identity
    let to_balance = mercury_client.get_balance(payload_to).unwrap();
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u64.to_string());
}
