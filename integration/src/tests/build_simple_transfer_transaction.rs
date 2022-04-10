use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    issue_udt_with_cheque, prepare_acp, prepare_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, SimpleTransferPayload, ToInfo,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_simple_transfer_ckb",
    test_fn: test_simple_transfer_ckb
});
fn test_simple_transfer_ckb() {
    let (from_address, from_pk) =
        prepare_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![from_address.to_string()],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u64.to_string(),
        }],
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &from_pk).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx);

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u64.to_string());

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.parse::<u64>().unwrap();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert!(from_left_capacity < 100_0000_0000);
    assert!(from_left_capacity > 99_0000_0000);
}

inventory::submit!(IntegrationTest {
    name: "test_simple_transfer_udt_hold_by_to",
    test_fn: test_simple_transfer_udt_hold_by_to
});
fn test_simple_transfer_udt_hold_by_to() {
    // issue udt with cheque
    let (sender_address, sender_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk) =
        prepare_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u64,
    );

    // new acp account for to
    let (to_address_secp, to_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_acp(&udt_hash, &to_address_secp, &to_address_pk).unwrap();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![receiver_address.to_string()],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100u64.to_string(),
        }],
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &receiver_address_pk).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx);

    // get balance of to address
    let to_identity = new_identity_from_secp_address(&to_address_secp.to_string()).unwrap();
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(to_identity.0)),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) =
        if to_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&to_balance.balances[0], &to_balance.balances[1])
        } else {
            (&to_balance.balances[1], &to_balance.balances[0])
        };
    assert_eq!(to_balance.balances.len(), 2);
    assert_ne!(ckb_balance.free, 108u64.to_string());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u64.to_string());
    assert_eq!(udt_balance.free, 100u64.to_string());

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.parse::<u64>().unwrap();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert!(from_left_capacity < 100_0000_0000);
    assert!(from_left_capacity > 99_0000_0000);
}

inventory::submit!(IntegrationTest {
    name: "test_simple_transfer_udt_hold_by_from",
    test_fn: test_simple_transfer_udt_hold_by_from
});
fn test_simple_transfer_udt_hold_by_from() {
    // issue udt with cheque
    let (sender_address, sender_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u64,
    );

    // prepare address for from
    let (from_address, from_address_pk) = (receiver_address, receiver_address_pk);

    // prepare address for to
    let (to_address_secp, _to_address_pk) = generate_rand_secp_address_pk_pair();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![from_address.to_string()],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100u64.to_string(),
        }],
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &from_address_pk).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx);

    // get balance of to address
    let to_identity = new_identity_from_secp_address(&to_address_secp.to_string()).unwrap();
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(to_identity.0)),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u64.to_string());

    // get balance of from address
    let from_identity = new_identity_from_secp_address(&from_address.to_string()).unwrap();
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(from_identity.0)),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].occupied,
        162_0000_0000u64.to_string()
    );
}
