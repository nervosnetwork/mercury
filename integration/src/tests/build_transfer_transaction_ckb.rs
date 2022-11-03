use super::IntegrationTest;
use crate::const_definition::{MERCURY_URI, UDT_1_HASH};
use crate::utils::address::{
    acp::build_acp_address, generate_rand_secp_address_pk_pair, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, OutputCapacityProvider, PayFee, ToInfo,
    TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_from_provide_capacity",
    test_fn: test_transfer_ckb_from_provide_capacity
});
fn test_transfer_ckb_from_provide_capacity() {
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],
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
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx);

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

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert!(100_0000_0000u128 > from_left_capacity);
    assert!(99_0000_0000u128 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_to_provide_capacity",
    test_fn: test_transfer_ckb_to_provide_capacity
});
fn test_transfer_ckb_to_provide_capacity() {
    // get udt_hash
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    // prepare from
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare 200 ckb");

    // new acp account for to
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

    // build tx
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert!(100_0000_0000u128 > from_left_capacity);
    assert!(99_0000_0000u128 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_change_to_new_output",
    test_fn: test_change_to_new_output
});
fn test_change_to_new_output() {
    // prepare ckb
    let (from_address_1, from_pk_1, _out_point_1) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (from_address_2, from_pk_2, out_point_2) =
        prepare_secp_address_with_ckb_capacity(400_0000_0000).expect("prepare ckb");
    let (from_address_3, from_pk_3, _out_point_3) =
        prepare_secp_address_with_ckb_capacity(650_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // prepare acp
    issue_udt_1().unwrap();
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address_1,
        &from_address_1,
        &from_pk_1,
        Some(1),
    )
    .unwrap();
    let from_acp_address_1 = build_acp_address(&from_address_1).unwrap();

    let pks = vec![from_pk_1, from_pk_2, from_pk_3];

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // get balance 1
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_1.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert!(107u128 < balance.balances[0].free.into());

    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_1.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert!(142u128 < balance.balances[0].occupied.into());

    // transfer
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![
            JsonItem::Address(from_address_1.to_string()), // 107+ free ckb
            JsonItem::OutPoint(out_point_2),               // 400 free ckb
            JsonItem::Address(from_address_3.to_string()), // 650 free ckb
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 400_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx.clone());

    // change is enough to build an output, so there is no need to put change into acp
    assert_eq!(2, tx.inputs.len());
    assert_eq!(2, tx.outputs.len());

    // get balance 2
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_2.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert!(107u128 < ckb_balance.free.into()); // new change cell
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_from_provide_capacity_out_point",
    test_fn: test_transfer_ckb_from_provide_capacity_out_point
});
fn test_transfer_ckb_from_provide_capacity_out_point() {
    let (from_1_address, from_1_pk, out_point_1) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");

    let (from_2_address, from_2_pk, _out_point_2) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::OutPoint(out_point_1.clone())],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 200_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let tx = mercury_client.build_transfer_transaction(payload);
    assert!(tx.is_err());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![
            JsonItem::OutPoint(out_point_1),
            JsonItem::Address(from_1_address.to_string()),
            JsonItem::Address(from_2_address.to_string()),
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 200_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_1_pk, from_2_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_2_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(199_0000_0000u128 < balance.balances[0].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_from_provide_capacity_to_acp_address",
    test_fn: test_transfer_ckb_from_provide_capacity_to_acp_address
});
fn test_transfer_ckb_from_provide_capacity_to_acp_address() {
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");

    let (to_address, _to_address_pk) = generate_rand_secp_address_pk_pair();
    let to_acp_address = build_acp_address(&to_address).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 200_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_acp_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(200_0000_0000u128, balance.balances[0].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_from_provide_capacity_pay_fee_to",
    test_fn: test_transfer_ckb_from_provide_capacity_pay_fee_to
});
fn test_transfer_ckb_from_provide_capacity_pay_fee_to() {
    // prepare from
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare 200 ckb");

    // prepare to
    let (to_address_secp, _to_address_pk) = generate_rand_secp_address_pk_pair();

    // build tx
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.into();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert_eq!(100_0000_0000u128, from_left_capacity);

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address_secp.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let to_capacity = balance.balances[0].free.into();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(100_0000_0000u128 > to_capacity);
    assert!(99_0000_0000u128 < to_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_to_provide_capacity_to_pay_fee",
    test_fn: test_transfer_ckb_to_provide_capacity_to_pay_fee
});
fn test_transfer_ckb_to_provide_capacity_to_pay_fee() {
    // get udt_hash
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();

    // prepare from
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare 200 ckb");

    // new acp account for to
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

    // build tx
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: None,
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of from address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let from_capacity = balance.balances[0].free.into();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(100_0000_0000u128, from_capacity);

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_acp_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let to_free_capacity: u128 = balance.balances[0].free.into();
    let to_occupied_capacity: u128 = balance.balances[0].occupied.into();
    assert_eq!(balance.balances.len(), 1);
    assert!(242_0000_0000u128 > to_free_capacity + to_occupied_capacity);
    assert!(241_0000_0000u128 < to_free_capacity + to_occupied_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_change_to_new_acp",
    test_fn: test_change_to_new_acp
});
fn test_change_to_new_acp() {
    // prepare ckb
    let (from_address_1, from_pk_1, _out_point_1) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (from_address_2, from_pk_2, _out_point_2) =
        prepare_secp_address_with_ckb_capacity(400_0000_0000).expect("prepare ckb");
    let (from_address_3, from_pk_3, _out_point_3) =
        prepare_secp_address_with_ckb_capacity(650_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // prepare acp 1
    issue_udt_1().unwrap();
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address_1,
        &from_address_1,
        &from_pk_1,
        Some(1),
    )
    .unwrap();
    let from_acp_address_1 = build_acp_address(&from_address_1).unwrap();

    // prepare acp 2
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address_2,
        &from_address_2,
        &from_pk_2,
        Some(1),
    )
    .unwrap();
    let from_acp_address_2 = build_acp_address(&from_address_2).unwrap();

    let pks = vec![from_pk_1, from_pk_2, from_pk_3];

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // get balance 1
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_1.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert!(107_0000_0000u128 < balance.balances[0].free.into());

    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_1.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(142_0000_0000u128, balance.balances[0].occupied.into());

    // transfer
    let identity_1 = new_identity_from_secp_address(&from_address_1.to_string()).unwrap();
    let identity_2 = new_identity_from_secp_address(&from_address_2.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![
            JsonItem::Identity(hex::encode(identity_1.0)), // 107+ free ckb + acp cell
            JsonItem::Identity(hex::encode(identity_2.0)), // 257+ free ckb + acp cell
            JsonItem::Address(from_address_3.to_string()), // 650 free ckb
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 350_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx.clone());

    // change is not enough to build an output, so change into acp(from db)
    assert_eq!(3, tx.inputs.len());
    assert_eq!(2, tx.outputs.len());

    // get balance 3
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_3.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(650_0000_0000u128, ckb_balance.free.into());

    // get balance 2 acp
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_2.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(142_0000_0000u128, ckb_balance.occupied.into());

    // get balance 1 acp
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_1.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(142_0000_0000u128, ckb_balance.occupied.into());
    assert!(15_0000_0000u128 < ckb_balance.free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_change_need_pool",
    test_fn: test_change_need_pool
});
fn test_change_need_pool() {
    // prepare ckb
    let (from_address_1, from_pk_1, _out_point_1) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare ckb");
    let (from_address_2, from_pk_2, _out_point_2) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare ckb");
    let (from_address_3, from_pk_3, _out_point_3) =
        prepare_secp_address_with_ckb_capacity(100_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let pks = vec![from_pk_1, from_pk_2, from_pk_3];

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![
            JsonItem::Address(from_address_1.to_string()), // 100 free ckb
            JsonItem::Address(from_address_2.to_string()), // 100 free ckb
            JsonItem::Address(from_address_3.to_string()), // 100 free ckb
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 195_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx.clone());

    // change is not enough to build an output
    // no acp
    // so need pool a cell from db, build change cell
    assert_eq!(3, tx.inputs.len());
    assert_eq!(2, tx.outputs.len());

    // get balance 2
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_2.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert!(104_0000_0000u128 < ckb_balance.free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_change_to_output_acp",
    test_fn: test_change_to_output_acp
});
fn test_change_to_output_acp() {
    // prepare ckb
    let (from_address_1, from_pk_1, _out_point_1) =
        prepare_secp_address_with_ckb_capacity(145_0000_0000).expect("prepare ckb");
    let (from_address_2, from_pk_2, _out_point_2) =
        prepare_secp_address_with_ckb_capacity(145_0000_0000).expect("prepare ckb");
    let (from_address_3, from_pk_3, _out_point_3) =
        prepare_secp_address_with_ckb_capacity(65_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // prepare acp 1
    issue_udt_1().unwrap();
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address_1,
        &from_address_1,
        &from_pk_1,
        Some(1),
    )
    .unwrap();
    let from_acp_address_1 = build_acp_address(&from_address_1).unwrap();

    // prepare acp 2
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address_2,
        &from_address_2,
        &from_pk_2,
        Some(1),
    )
    .unwrap();
    let from_acp_address_2 = build_acp_address(&from_address_2).unwrap();

    let pks = vec![from_pk_1, from_pk_2, from_pk_3];

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer
    let identity_1 = new_identity_from_secp_address(&from_address_1.to_string()).unwrap();
    let identity_2 = new_identity_from_secp_address(&from_address_2.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![
            JsonItem::Identity(hex::encode(identity_1.0)), // acp cell, free: 3-
            JsonItem::Identity(hex::encode(identity_2.0)), // acp cell, free: 3-
            JsonItem::Address(from_address_3.to_string()), // secp cell, free: 65
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 61_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx.clone());

    // change is not enough to build an output, so change into output acp(search order: rev)
    assert_eq!(3, tx.inputs.len());
    assert_eq!(3, tx.outputs.len());

    // get balance 3
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_3.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    assert_eq!(balance.balances.len(), 0);

    // get balance 2 acp
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_2.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(142_0000_0000u128, ckb_balance.occupied.into());
    assert!(9_0000_0000u128 < ckb_balance.free.into());

    // get balance 1 acp
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_acp_address_1.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(142_0000_0000u128, ckb_balance.occupied.into());
    assert_eq!(0u128, ckb_balance.free.into());
}
