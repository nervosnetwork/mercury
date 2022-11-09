use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::secp::{
    generate_rand_secp_address_pk_pair, prepare_secp_address_with_ckb_capacity,
};
use crate::utils::address::{
    acp::build_acp_address, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{issue_udt_with_cheque, prepare_account, send_transaction_to_ckb};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, GetBalancePayload, JsonItem, OutputCapacityProvider, SudtIssuePayload, ToInfo,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_issue_udt_from_provide_capacity",
    test_fn: test_issue_udt_from_provide_capacity
});
fn test_issue_udt_from_provide_capacity() {
    // prepare
    let (owner_address, owner_pk, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // issue udt
    let _tx_hash = issue_udt_with_cheque(&owner_address, &owner_pk, &to_address, 100u128);

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // get balance of to identity, AssetType::UDT
    let to_identity = new_identity_from_secp_address(&to_address.to_string()).unwrap();
    let udt_hash = get_udt_hash_by_owner(&owner_address).unwrap();
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash));
    let payload_to = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload_to).unwrap();
    let udt_balance = &to_balance.balances[0];

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(udt_balance.free, 100u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_issue_udt_from_multi_item",
    test_fn: test_issue_udt_from_multi_item
});
fn test_issue_udt_from_multi_item() {
    // prepare from items
    let (owner_address, owner_pk, _) =
        prepare_secp_address_with_ckb_capacity(145_0000_0000).expect("prepare ckb");
    let (_from_address_1, from_address_pk_1, out_point_1) =
        prepare_secp_address_with_ckb_capacity(145_0000_0000).expect("prepare ckb");
    let (from_address_2, from_address_pk_2, _) =
        prepare_secp_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let pks = vec![owner_pk, from_address_pk_1, from_address_pk_2];

    // prepare to items
    let (to_address_1, to_address_pk_1, _) =
        prepare_secp_address_with_ckb_capacity(150_0000_0000).expect("prepare ckb");
    let (to_address_2, _to_address_pk_2) = generate_rand_secp_address_pk_pair();

    // issue udt
    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        from: vec![
            JsonItem::OutPoint(out_point_1),
            JsonItem::Address(owner_address.to_string()),
            JsonItem::Address(from_address_2.to_string()),
        ],
        to: vec![
            ToInfo {
                address: to_address_1.to_string(),
                amount: 100.into(),
            },
            ToInfo {
                address: to_address_2.to_string(),
                amount: 100.into(),
            },
        ],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        fee_rate: None,
        since: None,
    };

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // build tx
    let tx = mercury_client
        .build_sudt_issue_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();

    // send tx to ckb node
    send_transaction_to_ckb(tx).unwrap();

    // get balance of to identity 1, AssetType::UDT
    let to_identity = new_identity_from_secp_address(&to_address_1.to_string()).unwrap();
    let udt_hash = get_udt_hash_by_owner(&owner_address).unwrap();
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash.clone()));
    let payload_to = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload_to.clone()).unwrap();
    let udt_balance = &to_balance.balances[0];

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(udt_balance.free, 100u128.into());

    // additional issue udt for to acp 1
    prepare_account(
        &udt_hash,
        &to_address_1,
        &to_address_1,
        &to_address_pk_1,
        Some(1),
    )
    .unwrap();
    let to_acp_address_1 = build_acp_address(&to_address_1).unwrap();

    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        from: vec![JsonItem::Address(owner_address.to_string())],
        to: vec![ToInfo {
            address: to_acp_address_1.to_string(),
            amount: 100.into(),
        }],
        output_capacity_provider: None,
        fee_rate: None,
        since: None,
    };

    // build tx
    let tx = mercury_client
        .build_sudt_issue_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();

    // send tx to ckb node
    send_transaction_to_ckb(tx).unwrap();

    // get balance of to identity, AssetType::UDT
    let to_balance = mercury_client.get_balance(payload_to).unwrap();
    let udt_balance = &to_balance.balances;

    assert_eq!(to_balance.balances.len(), 2);
    assert_eq!(udt_balance[0].free, 100u128.into());
    assert_eq!(udt_balance[1].free, 100u128.into());
}
