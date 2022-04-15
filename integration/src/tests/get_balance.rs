use super::IntegrationTest;
use crate::const_definition::{
    CHEQUE_LOCK_EPOCH, GENESIS_BUILT_IN_ADDRESS_1, MERCURY_URI, UDT_1_HOLDER_ACP_ADDRESS,
};
use crate::utils::address::build_cheque_address;
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    fast_forward_epochs, issue_udt_with_cheque, prepare_address_with_ckb_capacity,
};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, AssetType, GetBalancePayload, JsonItem};

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
    assert_eq!(response.balances[0].asset_info.asset_type, AssetType::CKB);
    println!("GENESIS_BUILT_IN_ADDRESS_1:");
    println!("  free: {:?}", response.balances[0].free);
    println!("  occupied: {:?}", response.balances[0].occupied);
    println!("  frozen: {:?}", response.balances[0].frozen);
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance_of_udt_1_holder_address",
    test_fn: test_get_balance_of_udt_1_holder_address
});
fn test_get_balance_of_udt_1_holder_address() {
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Address(UDT_1_HOLDER_ACP_ADDRESS.get().unwrap().to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = if balance.balances[0].asset_info.asset_type == AssetType::CKB
    {
        (&balance.balances[0], &balance.balances[1])
    } else {
        (&balance.balances[1], &balance.balances[0])
    };
    assert_eq!(balance.balances.len(), 2);
    println!("UDT_1_HOLDER_ACP_ADDRESS:");
    println!("  ckb free: {:?}", ckb_balance.free);
    println!("  ckb occupied: {:?}", ckb_balance.occupied);
    println!("  ckb frozen: {:?}", ckb_balance.frozen);
    println!("  udt free: {:?}", udt_balance.free);
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance_of_item_has_cheque",
    test_fn: test_get_balance_of_item_has_cheque
});
fn test_get_balance_of_item_has_cheque() {
    // prepare
    let (sender_address, sender_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (receiver_address, _receiver_address_pk) = generate_rand_secp_address_pk_pair();

    // issue udt
    let tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    )
    .unwrap();

    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let sender_identity = new_identity_from_secp_address(&sender_address.to_string()).unwrap();
    let receiver_identity = new_identity_from_secp_address(&receiver_address.to_string()).unwrap();

    // get balance of to identity, AssetType::UDT
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash));
    let payload_receiver = GetBalancePayload {
        item: JsonItem::Identity(receiver_identity.encode()),
        asset_infos,
        tip_block_number: None,
    };
    let receiver_balance = mercury_client
        .get_balance(payload_receiver.clone())
        .unwrap();
    let udt_balance = &receiver_balance.balances[0];
    assert_eq!(receiver_balance.balances.len(), 1);
    assert_eq!(udt_balance.free, 100u128.into());

    // get balance of to identity, AssetType::CKB
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Identity(receiver_identity.encode()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 0);

    // get balance of to identity, HashSet::new()
    let payload = GetBalancePayload {
        item: JsonItem::Identity(receiver_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u128.into());

    // get balance of sender identity
    let payload_sender = GetBalancePayload {
        item: JsonItem::Identity(sender_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let sender_balance = mercury_client.get_balance(payload_sender.clone()).unwrap();
    let sender_left_capacity = sender_balance.balances[0].free.into();
    assert_eq!(sender_balance.balances.len(), 1);
    assert!(88_0000_0000u128 > sender_left_capacity);
    assert!(87_0000_0000u128 < sender_left_capacity);

    // get balance of out point of cheque
    let tx_info = mercury_client
        .get_transaction_info(tx_hash)
        .unwrap()
        .transaction
        .unwrap();
    let out_point = &tx_info
        .records
        .iter()
        .find(|record| record.asset_info.asset_type == AssetType::UDT)
        .unwrap()
        .out_point;
    let payload_out_point = GetBalancePayload {
        item: JsonItem::OutPoint(out_point.to_owned()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload_out_point).unwrap();
    let (ckb_balance, udt_balance) = if balance.balances[0].asset_info.asset_type == AssetType::CKB
    {
        (&balance.balances[0], &balance.balances[1])
    } else {
        (&balance.balances[1], &balance.balances[0])
    };
    assert_eq!(ckb_balance.occupied, 162_0000_0000u128.into());
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(udt_balance.free, 100u128.into());

    // get balance of address of cheque
    let cheque_address = build_cheque_address(&receiver_address, &sender_address).unwrap();
    let payload_out_point = GetBalancePayload {
        item: JsonItem::Address(cheque_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload_out_point).unwrap();
    let (ckb_balance, udt_balance) = if balance.balances[0].asset_info.asset_type == AssetType::CKB
    {
        (&balance.balances[0], &balance.balances[1])
    } else {
        (&balance.balances[1], &balance.balances[0])
    };
    assert_eq!(ckb_balance.occupied, 162_0000_0000u128.into());
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(udt_balance.free, 100u128.into());

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // get balance of sender identity
    let sender_balance = mercury_client.get_balance(payload_sender).unwrap();
    let (ckb_balance, udt_balance) =
        if sender_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&sender_balance.balances[0], &sender_balance.balances[1])
        } else {
            (&sender_balance.balances[1], &sender_balance.balances[0])
        };

    assert_eq!(sender_balance.balances.len(), 2);
    assert_ne!(ckb_balance.free, 0u128.into());
    assert_eq!(ckb_balance.occupied, 162_0000_0000u128.into());
    assert_eq!(udt_balance.free, 100u128.into());

    // get balance of to identity
    let receiver_balance = mercury_client.get_balance(payload_receiver).unwrap();
    assert_eq!(receiver_balance.balances.len(), 1);
    assert_eq!(receiver_balance.balances[0].free, 100u128.into());
}
