use super::IntegrationTest;
use crate::const_definition::{
    CHEQUE_LOCK_EPOCH, MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS,
    UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::{
    build_acp_address, build_cheque_address, build_secp_address,
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    fast_forward_epochs, issue_udt_1, issue_udt_with_cheque, prepare_account,
    prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::{sign_transaction, sign_transaction_for_cheque_of_sender};

use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, OutputCapacityProvider, PayFee, ToInfo,
    TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_identity_has_in_lock_cheque",
    test_fn: test_transfer_udt_to_provide_capacity_from_identity_has_in_lock_cheque
});
fn test_transfer_udt_to_provide_capacity_from_identity_has_in_lock_cheque() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer cheque udt from sender
    let udt_identity = new_identity_from_secp_address(&sender_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.clone()),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload);
    assert!(tx.is_err());
    if let Err(e) = tx {
        assert!(e.to_string().contains("Required UDT is not enough"))
    }

    // transfer cheque udt from receiver
    let udt_identity = new_identity_from_secp_address(&receiver_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of udt_address
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();
    assert_eq!(from_balance.balances.len(), 1);
    assert!(100_0000_0000u128 > from_left_capacity);
    assert!(99_0000_0000u128 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_sender_cheque",
    test_fn: test_transfer_udt_to_provide_capacity_from_sender_cheque
});
fn test_transfer_udt_to_provide_capacity_from_sender_cheque() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, _receiver_address_pk) = generate_rand_secp_address_pk_pair();
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // transfer udt
    let udt_owner_identity = new_identity_from_secp_address(&sender_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_owner_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction_for_cheque_of_sender(tx, &sender_address_pk, vec![1]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of udt_address
    let payload = GetBalancePayload {
        item: JsonItem::Address(sender_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();
    assert_eq!(from_balance.balances.len(), 1);
    assert!(250_0000_0000u128 > from_left_capacity);
    assert!(249_0000_0000u128 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_receiver_cheque",
    test_fn: test_transfer_udt_to_provide_capacity_from_receiver_cheque
});
fn test_transfer_udt_to_provide_capacity_from_receiver_cheque() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // transfer udt
    let udt_identity = new_identity_from_secp_address(&receiver_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of udt_address
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();
    assert_eq!(from_balance.balances.len(), 1);
    assert!(100_0000_0000u128 > from_left_capacity);
    assert!(99_0000_0000u128 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_receiver_cheque_change_udt",
    test_fn: test_transfer_udt_to_provide_capacity_from_receiver_cheque_change_udt
});
fn test_transfer_udt_to_provide_capacity_from_receiver_cheque_change_udt() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(143_0000_0000).expect("prepare 143 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer cheque udt from receiver
    let udt_identity = new_identity_from_secp_address(&receiver_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of receiver
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&from_balance.balances[0], &from_balance.balances[1]);
    assert_eq!(from_balance.balances.len(), 2);
    assert_ne!(ckb_balance.free, 0u128.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 20u128.into());

    // get balance of sender
    let payload = GetBalancePayload {
        item: JsonItem::Address(sender_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_receiver_has_cheque_change_udt_to_acp",
    test_fn: test_transfer_udt_to_provide_capacity_from_receiver_has_cheque_change_udt_to_acp
});
fn test_transfer_udt_to_provide_capacity_from_receiver_has_cheque_change_udt_to_acp() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(145_0000_0000).expect("prepare 145 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new acp account for receiver
    prepare_account(
        &udt_hash,
        &receiver_address,
        &receiver_address,
        &receiver_address_pk,
        Some(1),
    )
    .unwrap();

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_address_acp = build_acp_address(&to_address_secp).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer cheque udt from receiver
    let udt_identity = new_identity_from_secp_address(&receiver_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_address_acp.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of receiver
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(udt_identity.0)),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&from_balance.balances[0], &from_balance.balances[1]);
    assert_eq!(from_balance.balances.len(), 2);
    assert_ne!(ckb_balance.free, 0u128.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 20u128.into());

    // get balance of sender
    let payload = GetBalancePayload {
        item: JsonItem::Address(sender_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_out_point_cheque_part_claim",
    test_fn: test_transfer_udt_to_provide_capacity_from_out_point_cheque_part_claim
});
fn test_transfer_udt_to_provide_capacity_from_out_point_cheque_part_claim() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, receiver_ckb_out_point) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    )
    .unwrap();
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx_info = mercury_client
        .get_transaction_info(tx_hash)
        .unwrap()
        .transaction
        .unwrap();
    let cheque_out_point = &tx_info
        .records
        .iter()
        .find(|record| record.asset_info.asset_type == AssetType::UDT)
        .unwrap()
        .out_point;

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // transfer cheque udt from receiver
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![
            JsonItem::OutPoint(cheque_out_point.to_owned()),
            JsonItem::OutPoint(receiver_ckb_out_point),
        ],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of receiver
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let capacity = balance.balances[0].free.into();
    assert_eq!(balance.balances.len(), 1);
    assert!(100_0000_0000u128 > capacity);
    assert!(99_0000_0000u128 < capacity);

    // get balance of sender
    let payload = GetBalancePayload {
        item: JsonItem::Address(sender_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

    assert_eq!(balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 20u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_cheque_address_part_claim",
    test_fn: test_transfer_udt_to_provide_capacity_from_cheque_address_part_claim
});
fn test_transfer_udt_to_provide_capacity_from_cheque_address_part_claim() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, receiver_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    )
    .unwrap();
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let cheque_address = build_cheque_address(&receiver_address, &sender_address).unwrap();

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // transfer cheque udt from receiver
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![
            JsonItem::Address(cheque_address.to_string()),
            JsonItem::Address(receiver_address.to_string()),
        ],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[receiver_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of receiver
    let payload = GetBalancePayload {
        item: JsonItem::Address(receiver_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let capacity = balance.balances[0].free.into();
    assert_eq!(balance.balances.len(), 1);
    assert!(100_0000_0000u128 > capacity);
    assert!(99_0000_0000u128 < capacity);

    // get balance of sender
    let payload = GetBalancePayload {
        item: JsonItem::Address(sender_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

    assert_eq!(balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 20u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_from_provide_capacity_acp",
    test_fn: test_transfer_udt_from_provide_capacity_acp
});
fn test_transfer_udt_from_provide_capacity_acp() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    // prepare to address
    let (to_address_secp, _to_address_pk) = generate_rand_secp_address_pk_pair();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // transfer cheque udt from receiver
    let from_identity = new_identity_from_secp_address(&acp_address_with_udt.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[acp_address_pk.to_owned()]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of to address
    let to_identity = new_identity_from_secp_address(&to_address_secp.to_string()).unwrap();
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(to_identity.0)),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&to_balance.balances[0], &to_balance.balances[1]);

    assert_eq!(to_balance.balances.len(), 2);
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 80u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_provide_capacity_from_sender_has_cheque_part_withdraw",
    test_fn: test_transfer_udt_to_provide_capacity_from_sender_has_cheque_part_withdraw
});
fn test_transfer_udt_to_provide_capacity_from_sender_has_cheque_part_withdraw() {
    // issue udt with cheque
    let (sender_address, sender_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();
    let (receiver_address, _receiver_address_pk) = generate_rand_secp_address_pk_pair();

    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );

    // new acp account for to
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        &udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_address_acp = build_acp_address(&to_address_secp).unwrap();

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer cheque udt
    let udt_identity = new_identity_from_secp_address(&sender_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
        to: vec![ToInfo {
            address: to_address_acp.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction_for_cheque_of_sender(tx, &sender_address_pk, vec![1]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of sender
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(udt_identity.0)),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&balance.balances[0], &balance.balances[1]);

    assert_eq!(balance.balances.len(), 2);
    assert!(107_0000_0000u128 < ckb_balance.free.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 20u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_pay_fee_to",
    test_fn: test_transfer_udt_pay_fee_to
});
fn test_transfer_udt_pay_fee_to() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let from_identity = new_identity_from_secp_address(&acp_address_with_udt.to_string()).unwrap();
    let from_secp_address = build_secp_address(acp_address_with_udt).unwrap();

    // prepare to address
    let (to_address_secp, to_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_account(
        udt_hash,
        &to_address_secp,
        &to_address_secp,
        &to_address_pk,
        Some(1),
    )
    .unwrap();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // transfer cheque udt from receiver
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload);

    // when transferring udt, PayFee::To is meaningless and will fail
    assert!(tx.is_err());
    if let Err(e) = tx {
        assert!(e.to_string().contains("failed to pay fee by to"));
    }

    let cheque_address = build_cheque_address(&to_address_secp, &from_secp_address).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: cheque_address.to_string(),
            amount: 80u128.into(),
        }],

        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload);

    // when transferring udt, PayFee::To is meaningless and will fail
    assert!(tx.is_err());
    if let Err(e) = tx {
        assert!(e.to_string().contains("failed to pay fee by to"));
    }

    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload);

    // when transferring udt, PayFee::To is meaningless and will fail
    assert!(tx.is_err());
    if let Err(e) = tx {
        assert!(e.to_string().contains("failed to pay fee by to"));
    }
}
