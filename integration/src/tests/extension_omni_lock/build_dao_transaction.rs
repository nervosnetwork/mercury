use super::super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::omni_lock::prepare_omni_secp_address_with_capacity;
use crate::utils::address::secp::generate_rand_secp_address_pk_pair;
use crate::utils::instruction::{fast_forward_epochs, send_transaction_to_ckb};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload,
    GetBalancePayload, JsonItem, OutputCapacityProvider, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_omni_dao_by_address",
    test_fn: test_omni_dao_by_address
});
fn test_omni_dao_by_address() {
    let (_, address, address_pk, _) =
        prepare_omni_secp_address_with_capacity(300_0000_0000).expect("prepare ckb");
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

    // get balance of address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());

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
    assert!(balance.balances[0].free > 99_0000_0000u128.into());
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());
    assert!(balance.balances[0].frozen > 98_0000_0000u128.into());
    assert!(balance.balances[0].frozen < 99_0000_0000u128.into());

    // claim
    fast_forward_epochs(176).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance.balances[0].free > 300_0000_0000u128.into());
    assert!(balance.balances[0].free < 301_0000_0000u128.into());
    assert_eq!(balance.balances[0].occupied, 0u128.into());
    assert_eq!(balance.balances[0].frozen, 0u128.into());

    let tx = mercury_client
        .build_dao_claim_transaction(claim_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance.balances[0].free > 300_0000_0000u128.into());
    assert!(balance.balances[0].free < 301_0000_0000u128.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_dao_pool_money",
    test_fn: test_omni_dao_pool_money
});
fn test_omni_dao_pool_money() {
    let (_, address, address_pk, _) =
        prepare_omni_secp_address_with_capacity(300_0000_0000).expect("prepare ckb");
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
    let tx = mercury_client.build_dao_claim_transaction(claim_payload);
    assert!(tx.is_err());

    fast_forward_epochs(4).unwrap();

    // withdraw
    let tx = mercury_client
        .build_dao_withdraw_transaction(withdraw_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // transfer 200
    let (to_address, _) = generate_rand_secp_address_pk_pair();
    let transfer_payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(address.to_string())],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: 200_0000_0000u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(transfer_payload.clone());
    assert!(tx.is_err());

    // claim
    fast_forward_epochs(176).unwrap();

    let tx = mercury_client
        .build_transfer_transaction(transfer_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance.balances[0].free > 100_0000_0000u128.into());
}
