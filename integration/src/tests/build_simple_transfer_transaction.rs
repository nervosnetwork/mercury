use super::IntegrationTest;
use crate::const_definition::{
    MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{
    issue_udt_1, issue_udt_with_acp, prepare_account, prepare_secp_address_with_ckb_capacity,
    send_transaction_to_ckb,
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
    let (from_address, from_pk, _) =
        prepare_secp_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![from_address.to_string()],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
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
    name: "test_simple_transfer_udt_to_provide_capacity",
    test_fn: test_simple_transfer_udt_to_provide_capacity
});
fn test_simple_transfer_udt_to_provide_capacity() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

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
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![acp_address_with_udt.to_string()],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100u128.into(),
        }],
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &[acp_address_pk.to_owned()]).unwrap();

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
    let balance_ckb_secp = to_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::CKB && b.ownership == to_address_secp.to_string()
        })
        .unwrap();
    let balance_ckb_acp = to_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::CKB && b.ownership != to_address_secp.to_string()
        })
        .unwrap();
    let balance_udt_acp = to_balance
        .balances
        .iter()
        .find(|b| {
            b.asset_info.asset_type == AssetType::UDT && b.ownership != to_address_secp.to_string()
        })
        .unwrap();
    assert_eq!(to_balance.balances.len(), 3);
    assert!(107_0000_0000u128 < balance_ckb_secp.free.into());
    assert!(108_0000_0000u128 > balance_ckb_secp.free.into());
    assert_eq!(balance_ckb_acp.occupied, 142_0000_0000u128.into());
    assert_eq!(balance_udt_acp.free, 100u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_simple_transfer_udt_from_provide_capacity",
    test_fn: test_simple_transfer_udt_from_provide_capacity
});
fn test_simple_transfer_udt_from_provide_capacity() {
    // prepare address for from
    let (from_address, from_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(500_0000_0000).expect("prepare 500 ckb");
    let _tx = issue_udt_with_acp(&from_address, &from_address_pk, 100).unwrap();
    let udt_hash = get_udt_hash_by_owner(&from_address).unwrap();

    // prepare address for to
    let (to_address_secp, _to_address_pk) = generate_rand_secp_address_pk_pair();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![from_address.to_string()],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100u128.into(),
        }],
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &[from_address_pk.to_owned()]).unwrap();

    // send tx to ckb node
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
    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(to_balance.balances[0].free, 100u128.into());

    // get balance of from address
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let ckb_balance = &from_balance.balances[0];
    assert_eq!(from_balance.balances.len(), 1);
    assert!(195_0000_0000u128 < ckb_balance.free.into());
}
