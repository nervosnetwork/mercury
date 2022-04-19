use super::IntegrationTest;
use crate::const_definition::{
    MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::{generate_rand_secp_address_pk_pair, new_identity_from_secp_address};
use crate::utils::instruction::{
    issue_udt_1, prepare_acp, prepare_address_with_ckb_capacity, send_transaction_to_ckb,
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
            amount: 100_0000_0000u128.into(),
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
    name: "test_simple_transfer_udt_hold_by_to",
    test_fn: test_simple_transfer_udt_hold_by_to
});
fn test_simple_transfer_udt_hold_by_to() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    // new acp account for to
    let (to_address_secp, to_address_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    prepare_acp(udt_hash, &to_address_secp, &to_address_pk).unwrap();

    // build tx
    let payload = SimpleTransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![acp_address_with_udt.to_string()],
        to: vec![ToInfo {
            address: to_address_secp.to_string(),
            amount: 100u128.into(),
        }],
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, acp_address_pk).unwrap();

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
    assert_ne!(ckb_balance.free, 108u128.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 100u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_simple_transfer_udt_hold_by_from",
    test_fn: test_simple_transfer_udt_hold_by_from
});
fn test_simple_transfer_udt_hold_by_from() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    // prepare address for from
    let (from_address, from_address_pk) = (acp_address_with_udt, acp_address_pk);

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
        change: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client
        .build_simple_transfer_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, from_address_pk).unwrap();

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
    assert_eq!(to_balance.balances[0].free, 100u128.into());

    // get balance of from address
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, _udt_balance) =
        if from_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&from_balance.balances[0], &from_balance.balances[1])
        } else {
            (&from_balance.balances[1], &from_balance.balances[0])
        };
    assert_eq!(from_balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
}
