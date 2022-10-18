use super::IntegrationTest;
use crate::const_definition::{
    CHEQUE_LOCK_EPOCH, GENESIS_BUILT_IN_ADDRESS_1, MERCURY_URI, UDT_1_HOLDER_ACP_ADDRESS,
};
use crate::utils::address::cheque::build_cheque_address;
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    fast_forward_epochs, issue_udt_1, issue_udt_with_cheque,
    prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload, ExtraType,
    GetBalancePayload, JsonItem,
};

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
        extra: None,
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
    issue_udt_1().unwrap();

    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Address(UDT_1_HOLDER_ACP_ADDRESS.get().unwrap().to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

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
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
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
    let cheque_address = build_cheque_address(&receiver_address, &sender_address).unwrap();

    // get balance of to identity, AssetType::UDT
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash));
    let payload_receiver = GetBalancePayload {
        item: JsonItem::Identity(receiver_identity.encode()),
        asset_infos,
        extra: None,
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
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 0);

    // get balance of to identity, HashSet::new()
    let payload = GetBalancePayload {
        item: JsonItem::Identity(receiver_identity.encode()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u128.into());

    // get balance of sender identity
    let payload_sender = GetBalancePayload {
        item: JsonItem::Identity(sender_identity.encode()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let sender_balance = mercury_client.get_balance(payload_sender.clone()).unwrap();
    let sender_balance_ckb_secp = sender_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::CKB && b.ownership == sender_address.to_string()
        })
        .unwrap();
    let sender_balance_ckb_secp = sender_balance_ckb_secp.free.into();
    let sender_balance_ckb_cheque = sender_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::CKB && b.ownership == cheque_address.to_string()
        })
        .unwrap();
    let sender_balance_ckb_cheque = sender_balance_ckb_cheque.occupied.into();
    assert_eq!(sender_balance.balances.len(), 2);
    assert!(88_0000_0000u128 > sender_balance_ckb_secp);
    assert!(87_0000_0000u128 < sender_balance_ckb_secp);
    assert_eq!(162_0000_0000u128, sender_balance_ckb_cheque);

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
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload_out_point).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

    assert_eq!(ckb_balance.occupied, 162_0000_0000u128.into());
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(udt_balance.free, 100u128.into());

    // get balance of address of cheque
    let payload_out_point = GetBalancePayload {
        item: JsonItem::Address(cheque_address.to_string()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload_out_point).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

    assert_eq!(balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 162_0000_0000u128.into());
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(udt_balance.free, 100u128.into());

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // get balance of sender identity
    let sender_balance = mercury_client.get_balance(payload_sender).unwrap();
    let sender_balance_udt_cheque = sender_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::UDT && b.ownership == cheque_address.to_string()
        })
        .unwrap();
    assert_eq!(sender_balance.balances.len(), 3);
    assert_eq!(sender_balance_udt_cheque.free, 100u128.into());

    // get balance of to identity
    let receiver_balance = mercury_client.get_balance(payload_receiver).unwrap();
    assert_eq!(receiver_balance.balances.len(), 1);
    assert_eq!(receiver_balance.balances[0].free, 100u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_dao_get_balance",
    test_fn: test_dao_get_balance
});
fn test_dao_get_balance() {
    let (address, address_pk, _) =
        prepare_secp_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");
    let pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // deposit
    let payload = DaoDepositPayload {
        from: vec![JsonItem::Address(address.to_string())],
        to: None,
        amount: 200_0000_0000.into(),
        fee_rate: None,
    };
    let tx = mercury_client
        .build_dao_deposit_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of dao
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
        asset_infos,
        extra: Some(ExtraType::Dao),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());
    assert_eq!(balance.balances[0].free, 0u128.into());
    assert_eq!(balance.balances[0].frozen, 98_0000_0000u128.into());

    // withdraw
    let withdraw_payload = DaoWithdrawPayload {
        from: vec![JsonItem::Address(address.to_string())],
        fee_rate: None,
    };
    let tx = mercury_client.build_dao_withdraw_transaction(withdraw_payload.clone());
    assert!(tx.is_err());

    // claim
    let claim_payload = DaoClaimPayload {
        from: vec![JsonItem::Address(address.to_string())],
        to: None,
        fee_rate: None,
    };
    let tx = mercury_client.build_dao_claim_transaction(claim_payload.clone());
    assert!(tx.is_err());

    fast_forward_epochs(4).unwrap();

    // withdraw
    let tx = mercury_client
        .build_dao_withdraw_transaction(withdraw_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].free, 0u128.into());
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());
    assert!(balance.balances[0].frozen > 98_0000_0000u128.into());

    fast_forward_epochs(176).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance.balances[0].free > 200_0000_0000u128.into());
    assert_eq!(balance.balances[0].occupied, 0u128.into());
    assert_eq!(balance.balances[0].frozen, 0u128.into());

    // claim
    let tx = mercury_client
        .build_dao_claim_transaction(claim_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 0);
}
