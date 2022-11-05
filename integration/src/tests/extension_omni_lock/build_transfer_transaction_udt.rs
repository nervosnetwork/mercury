use super::super::IntegrationTest;
use crate::const_definition::{
    MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::omni_lock::{
    build_omni_acp_account_address, generate_omni_secp_address_pk_pair,
};
use crate::utils::instruction::{
    issue_udt_1, prepare_account, prepare_secp_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, GetBalancePayload, JsonItem, OutputCapacityProvider, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_to_omni_acp",
    test_fn: test_transfer_udt_to_omni_acp
});
fn test_transfer_udt_to_omni_acp() {
    // issue udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    let (secp_address, secp_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(1000_0000_0000).unwrap();
    let (omni_identity, omni_address, _omni_address_pk) = generate_omni_secp_address_pk_pair();
    let omni_account_address = build_omni_acp_account_address(&omni_address).unwrap();

    // acp number: 1
    prepare_account(
        udt_hash,
        &omni_address,
        &secp_address,
        &secp_address_pk,
        Some(1),
    )
    .unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(omni_address.to_string()),
        asset_infos: asset_infos.clone(),
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 0);

    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(omni_identity.0)),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(1_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());

    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![
            JsonItem::Address(acp_address_with_udt.to_string()),
            JsonItem::Address(secp_address.to_string()),
        ],
        to: vec![ToInfo {
            address: omni_account_address.to_string(),
            amount: 100u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::To),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[acp_address_pk.to_owned(), secp_address_pk]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(omni_identity.0)),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(1_0000_0000u128, response.balances[0].free.into());
    assert_eq!(146_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(100u128, response.balances[1].free.into());
}

inventory::submit!(IntegrationTest {
    name: "test_omni_transfer_udt_from_provide_capacity",
    test_fn: test_omni_transfer_udt_from_provide_capacity
});
fn test_omni_transfer_udt_from_provide_capacity() {}
