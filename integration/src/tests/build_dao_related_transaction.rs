use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::generate_rand_secp_address_pk_pair;
use crate::utils::instruction::{
    fast_forward_epochs, prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use ckb_jsonrpc_types::OutPoint;
use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload,
    GetBalancePayload, JsonItem, OutputCapacityProvider, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_dao_by_address",
    test_fn: test_dao_by_address
});
fn test_dao_by_address() {
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
    name: "test_dao_pool_money",
    test_fn: test_dao_pool_money
});
fn test_dao_pool_money() {
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

inventory::submit!(IntegrationTest {
    name: "test_dao_by_out_point",
    test_fn: test_dao_by_out_point
});
fn test_dao_by_out_point() {
    let (address_1, address_pk_1, out_point_1) =
        prepare_secp_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");
    let (address_2, address_pk_2, _) =
        prepare_secp_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");
    let pks = vec![address_pk_1, address_pk_2];

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // deposit 1
    let payload = DaoDepositPayload {
        from: vec![
            JsonItem::OutPoint(out_point_1),
            JsonItem::Address(address_1.to_string()),
        ],
        to: None,
        amount: 200_0000_0000.into(),
        fee_rate: None,
    };
    let tx = mercury_client
        .build_dao_deposit_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let tx_hash = send_transaction_to_ckb(tx).unwrap();
    let out_point_deposit_1 = OutPoint {
        tx_hash,
        index: 0.into(),
    };

    // deposit 2
    let payload = DaoDepositPayload {
        from: vec![JsonItem::Address(address_2.to_string())],
        to: None,
        amount: 200_0000_0000.into(),
        fee_rate: None,
    };
    let tx = mercury_client
        .build_dao_deposit_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let tx_hash = send_transaction_to_ckb(tx).unwrap();
    let out_point_deposit_2 = OutPoint {
        tx_hash,
        index: 0.into(),
    };

    // get balance of address 1
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload_1 = GetBalancePayload {
        item: JsonItem::Address(address_1.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client
        .get_balance(balance_payload_1.clone())
        .unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());

    // get balance of address 2
    let balance_payload_2 = GetBalancePayload {
        item: JsonItem::Address(address_2.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client
        .get_balance(balance_payload_2.clone())
        .unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());

    // withdraw
    let withdraw_payload = DaoWithdrawPayload {
        from: vec![
            JsonItem::OutPoint(out_point_deposit_1),
            JsonItem::OutPoint(out_point_deposit_2),
            JsonItem::Address(address_2.to_string()),
        ],
        fee_rate: None,
    };
    let tx = mercury_client.build_dao_withdraw_transaction(withdraw_payload.clone());
    assert!(tx.is_err());

    // claim
    let claim_payload = DaoClaimPayload {
        from: vec![
            JsonItem::Address(address_1.to_string()),
            JsonItem::Address(address_2.to_string()),
        ],
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

    fast_forward_epochs(176).unwrap();

    // claim
    let tx = mercury_client
        .build_dao_claim_transaction(claim_payload)
        .unwrap();
    let tx = sign_transaction(tx, &pks).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance 1
    let balance_1 = mercury_client.get_balance(balance_payload_1).unwrap();
    assert_eq!(balance_1.balances.len(), 1);
    assert_eq!(balance_1.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance_1.balances[0].free > 500_0000_0000u128.into());

    // get_balance 2
    let balance_2 = mercury_client.get_balance(balance_payload_2).unwrap();
    assert_eq!(balance_2.balances.len(), 1);
    assert_eq!(balance_2.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance_2.balances[0].free < 100_0000_0000u128.into());

    assert!(balance_1.balances[0].free > balance_2.balances[0].free);
}
