use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::instruction::{
    fast_forward_epochs, prepare_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, AssetType, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload, From,
    GetBalancePayload, JsonItem,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_dao",
    test_fn: test_dao
});
fn test_dao() {
    let (address, address_pk) =
        prepare_address_with_ckb_capacity(300_0000_0000).expect("prepare ckb");

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // deposit
    let payload = DaoDepositPayload {
        from: From {
            items: vec![JsonItem::Address(address.to_string())],
        },
        to: None,
        amount: 200_0000_0000.into(),
        fee_rate: None,
    };
    let tx = mercury_client
        .build_dao_deposit_transaction(payload)
        .unwrap();
    let tx = sign_transaction(tx, &address_pk).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let balance_payload = GetBalancePayload {
        item: JsonItem::Address(address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload.clone()).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(balance.balances[0].occupied, 102_0000_0000u128.into());

    // withdraw
    let withdraw_payload = DaoWithdrawPayload {
        from: JsonItem::Address(address.to_string()),
        pay_fee: None,
        fee_rate: None,
    };
    let tx = mercury_client.build_dao_withdraw_transaction(withdraw_payload.clone());
    assert!(tx.is_err());

    // claim
    let claim_payload = DaoClaimPayload {
        from: JsonItem::Address(address.to_string()),
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
    let tx = sign_transaction(tx, &address_pk).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // claim
    fast_forward_epochs(176).unwrap();

    let tx = mercury_client
        .build_dao_claim_transaction(claim_payload.clone())
        .unwrap();
    let tx = sign_transaction(tx, &address_pk).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get_balance
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert!(balance.balances[0].free > 300_0000_0000u128.into());
}
