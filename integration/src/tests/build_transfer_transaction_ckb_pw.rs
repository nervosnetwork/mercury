use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::generate_rand_secp_address_pk_pair;
use crate::utils::instruction::{prepare_pw_address_with_capacity, send_transaction_to_ckb};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, OutputCapacityProvider, PayFee, ToInfo,
    TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_from_provide_capacity_pw",
    test_fn: test_transfer_ckb_from_provide_capacity_pw
});
fn test_transfer_ckb_from_provide_capacity_pw() {
    let (from_address, from_pk, _) =
        prepare_pw_address_with_capacity(200_0000_0000).expect("prepare ckb");
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
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

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
    name: "test_transfer_ckb_from_provide_capacity_pw_pay_fee_to",
    test_fn: test_transfer_ckb_from_provide_capacity_pw_pay_fee_to
});
fn test_transfer_ckb_from_provide_capacity_pw_pay_fee_to() {
    let (from_address, from_pk, _) =
        prepare_pw_address_with_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(from_address.to_string())],

        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 100_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: Some(PayFee::To),
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[from_pk]).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

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
    assert!(100_0000_0000u128 > to_balance.balances[0].free.into());
    assert!(99_0000_0000u128 < to_balance.balances[0].free.into());

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
}
