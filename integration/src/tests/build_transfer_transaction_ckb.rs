use super::IntegrationTest;
use crate::const_definition::{MERCURY_URI, UDT_1_HASH};
use crate::utils::address::{build_acp_address, generate_rand_secp_address_pk_pair};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, From, GetBalancePayload, JsonItem, Mode, To, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_from",
    test_fn: test_transfer_ckb_hold_by_from
});
fn test_transfer_ckb_hold_by_from() {
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(from_address.to_string())],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 100_0000_0000u128.into(),
            }],
            mode: Mode::HoldByFrom,
        },
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
    name: "test_transfer_ckb_hold_by_to",
    test_fn: test_transfer_ckb_hold_by_to
});
fn test_transfer_ckb_hold_by_to() {
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

    // build tx
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(from_address.to_string())],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address_secp.to_string(),
                amount: 100_0000_0000u128.into(),
            }],
            mode: Mode::HoldByTo,
        },
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
    name: "test_change",
    test_fn: test_change
});
fn test_change() {
    // prepare ckb
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(650_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // prepare acp
    issue_udt_1().unwrap();
    prepare_account(
        UDT_1_HASH.get().unwrap(),
        &from_address,
        &from_address,
        &from_pk,
        Some(1),
    )
    .unwrap();

    // get balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &balance.balances[0];
    assert_eq!(balance.balances.len(), 1);
    assert!(500u128 < ckb_balance.free.into());

    // transfer
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(from_address.to_string())],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 400_0000_0000u128.into(),
            }],
            mode: Mode::HoldByFrom,
        },
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx.clone());

    // change is enough to build an output, so there is no need to put change into acp
    assert_eq!(1, tx.inputs.len());
    assert_eq!(2, tx.outputs.len());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_from_out_point",
    test_fn: test_transfer_ckb_hold_by_from_out_point
});
fn test_transfer_ckb_hold_by_from_out_point() {
    let (_from_1_address, from_1_pk, out_point_1) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");

    let (from_2_address, from_2_pk, out_point_2) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::OutPoint(out_point_1.clone())],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 200_0000_0000u128.into(),
            }],
            mode: Mode::HoldByFrom,
        },
        pay_fee: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let tx = mercury_client.build_transfer_transaction(payload);
    assert!(tx.is_err());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![
                JsonItem::OutPoint(out_point_1),
                JsonItem::OutPoint(out_point_2),
            ],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 200_0000_0000u128.into(),
            }],
            mode: Mode::HoldByFrom,
        },
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
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(199_0000_0000u128 < balance.balances[0].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_from_to_acp_address",
    test_fn: test_transfer_ckb_hold_by_from_to_acp_address
});
fn test_transfer_ckb_hold_by_from_to_acp_address() {
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");

    let (to_address, _to_address_pk) = generate_rand_secp_address_pk_pair();
    let to_acp_address = build_acp_address(&to_address).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(from_address.to_string())],
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_acp_address.to_string(),
                amount: 200_0000_0000u128.into(),
            }],
            mode: Mode::HoldByFrom,
        },
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
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(200_0000_0000u128, balance.balances[0].free.into());
}
