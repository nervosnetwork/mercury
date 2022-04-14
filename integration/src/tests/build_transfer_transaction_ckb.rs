use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::{generate_rand_secp_address_pk_pair, get_udt_hash_by_owner};
use crate::utils::instruction::{
    issue_udt_with_cheque, prepare_acp, prepare_address_with_ckb_capacity, send_transaction_to_ckb,
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
    let (from_address, from_pk) =
        prepare_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
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
        change: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
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
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u64.into());

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
    assert!(100_0000_0000u64 > from_left_capacity);
    assert!(99_0000_0000u64 < from_left_capacity);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_to",
    test_fn: test_transfer_ckb_hold_by_to
});
fn test_transfer_ckb_hold_by_to() {
    // issue udt with cheque
    let (sender_address, sender_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let (receiver_address, _receiver_address_pk) =
        prepare_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(
        &sender_address,
        &sender_address_pk,
        &receiver_address,
        100u128,
    );
    let udt_hash = get_udt_hash_by_owner(&sender_address).unwrap();

    // prepare from
    let (from_address, from_pk) =
        prepare_address_with_ckb_capacity(200_0000_0000).expect("prepare 200 ckb");

    // new acp account for to
    let (to_address_secp, to_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_acp(&udt_hash, &to_address_secp, &to_address_pk).unwrap();

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
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &from_pk).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx);

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
    assert!(100_0000_0000u64 > from_left_capacity);
    assert!(99_0000_0000u64 < from_left_capacity);
}
