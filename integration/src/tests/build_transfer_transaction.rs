use super::IntegrationTest;
use crate::const_definition::{CKB_URI, MERCURY_URI};
use crate::utils::address::generate_rand_secp_address_pk_pair;
use crate::utils::instruction::{
    aggregate_transactions_into_blocks, prepare_address_with_ckb_capacity,
};
use crate::utils::rpc_client::{CkbRpcClient, MercuryRpcClient};
use crate::utils::signer::Signer;

use ckb_jsonrpc_types::OutputsValidator;
use core_rpc_types::{
    AssetInfo, AssetType, From, GetBalancePayload, JsonItem, Mode, Ownership, Source, To, ToInfo,
    TransferPayload,
};

use std::collections::HashSet;

fn test_transfer_ckb_hold_by_from() {
    let (from_address_with_200_ckb, from_pk) =
        prepare_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: From {
            items: vec![JsonItem::Address(from_address_with_200_ckb.to_string())],
            source: Source::Free,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 100_0000_0000u64.to_string(),
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
    let tx = mercury_client.build_transfer_transaction(payload);
    let tx = tx.unwrap();
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &from_pk).unwrap();

    // send tx to ckb node
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let tx_hash = ckb_client
        .send_transaction(tx, OutputsValidator::Passthrough)
        .unwrap();
    println!("send tx: 0x{}", tx_hash.to_string());
    aggregate_transactions_into_blocks().unwrap();

    // get balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(
        to_balance.balances[0].ownership,
        Ownership::Address(to_address.to_string())
    );
    assert_eq!(to_balance.balances[0].asset_info.asset_type, AssetType::CKB);
    assert_eq!(to_balance.balances[0].free, 100_0000_0000u64.to_string());

    // get balance
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(from_address_with_200_ckb.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.parse::<u64>().unwrap();

    assert_eq!(from_balance.balances.len(), 1);
    assert_eq!(
        from_balance.balances[0].ownership,
        Ownership::Address(from_address_with_200_ckb.to_string())
    );
    assert_eq!(
        from_balance.balances[0].asset_info.asset_type,
        AssetType::CKB
    );
    assert!(from_left_capacity < 100_0000_0000);
    assert!(from_left_capacity > 99_0000_0000);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_from",
    test_fn: test_transfer_ckb_hold_by_from
});
