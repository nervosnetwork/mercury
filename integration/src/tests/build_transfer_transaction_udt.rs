use super::IntegrationTest;
use crate::const_definition::{CHEQUE_LOCK_EPOCH, MERCURY_URI};
use crate::utils::address::{get_udt_hash_by_owner, new_identity_from_secp_address};
use crate::utils::instruction::{
    fast_forward_epochs, issue_udt_with_cheque, prepare_address_with_acp,
    prepare_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::Signer;

use core_rpc_types::{
    AssetInfo, From, GetBalancePayload, JsonItem, Mode, Source, To, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_hold_by_to_from_in_lock_cheque",
    test_fn: test_transfer_udt_hold_by_to_from_in_lock_cheque
});
fn test_transfer_udt_hold_by_to_from_in_lock_cheque() {
    // issue udt with cheque
    let (udt_owner_address, udt_owner_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&udt_owner_address).unwrap();
    let (udt_address, udt_address_pk) =
        prepare_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(&udt_owner_address, &udt_owner_pk, &udt_address, 100u64);

    // new acp account for to
    let (to_address_secp, _to_address_pk) = prepare_address_with_acp(&udt_hash).unwrap();

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // transfer cheque udt from sender
    let udt_identity = new_identity_from_secp_address(&udt_owner_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.clone()),
        from: From {
            items: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
            source: Source::Claimable,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address_secp.to_string(),
                amount: 100u64.to_string(),
            }],
            mode: Mode::HoldByTo,
        },
        pay_fee: None,
        change: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload);

    assert!(tx.is_err());
    if let Err(e) = tx {
        assert!(e.to_string().contains("Required UDT is not enough"))
    }

    // transfer cheque udt from receiver
    let udt_identity = new_identity_from_secp_address(&udt_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: From {
            items: vec![JsonItem::Identity(hex::encode(udt_identity.0))],
            source: Source::Claimable,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address_secp.to_string(),
                amount: 100u64.to_string(),
            }],
            mode: Mode::HoldByTo,
        },
        pay_fee: None,
        change: None,
        fee_rate: None,
        since: None,
    };
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &udt_address_pk).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx);

    // get balance of udt_address
    let payload = GetBalancePayload {
        item: JsonItem::Address(udt_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.parse::<u64>().unwrap();
    assert_eq!(from_balance.balances.len(), 1);
    assert!(from_left_capacity < 100_0000_0000);
    assert!(from_left_capacity > 99_0000_0000);
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_udt_hold_by_to_from_unlock_cheque_of_sender",
    test_fn: test_transfer_udt_hold_by_to_from_unlock_cheque_of_sender
});
fn test_transfer_udt_hold_by_to_from_unlock_cheque_of_sender() {
    // issue udt with cheque
    let (udt_owner_address, udt_owner_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare 250 ckb");
    let udt_hash = get_udt_hash_by_owner(&udt_owner_address).unwrap();
    let (udt_address, _udt_address_pk) =
        prepare_address_with_ckb_capacity(100_0000_0000).expect("prepare 100 ckb");
    let _tx_hash = issue_udt_with_cheque(&udt_owner_address, &udt_owner_pk, &udt_address, 100u64);

    // new account for to
    let (to_address_secp, _to_address_pk) = prepare_address_with_acp(&udt_hash).unwrap();

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // transfer udt
    let udt_owner_identity =
        new_identity_from_secp_address(&udt_owner_address.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash),
        from: From {
            items: vec![JsonItem::Identity(hex::encode(udt_owner_identity.0))],
            source: Source::Free,
        },
        to: To {
            to_infos: vec![ToInfo {
                address: to_address_secp.to_string(),
                amount: 100u64.to_string(),
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
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &udt_owner_pk).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of udt_address
    let payload = GetBalancePayload {
        item: JsonItem::Address(udt_owner_address.to_string()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let from_balance = mercury_client.get_balance(payload).unwrap();
    let from_left_capacity = from_balance.balances[0].free.parse::<u64>().unwrap();
    assert_eq!(from_balance.balances.len(), 1);
    println!("{:?}", from_left_capacity);
    assert!(from_left_capacity < 250_0000_0000);
    assert!(from_left_capacity > 249_0000_0000);
}
