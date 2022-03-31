use super::IntegrationTest;
use crate::const_definition::{CHEQUE_LOCK_EPOCH, MERCURY_URI};
use crate::utils::address::{generate_rand_secp_address_pk_pair, new_identity_from_secp_address};
use crate::utils::instruction::{
    fast_forward_epochs, prepare_address_with_ckb_capacity, send_transaction_to_ckb,
};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::Signer;

use core_rpc_types::{
    AssetInfo, AssetType, GetBalancePayload, JsonItem, Mode, SudtIssuePayload, To, ToInfo,
};

use std::collections::HashSet;

fn test_issue_udt_hold_by_from() {
    // prepare
    let (owner_address, owner_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();
    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        to: To {
            to_infos: vec![ToInfo {
                address: to_address.to_string(),
                amount: 100u64.to_string(),
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
    let tx = mercury_client
        .build_sudt_issue_transaction(payload)
        .unwrap();
    let signer = Signer::default();
    let tx = signer.sign_transaction(tx, &owner_pk).unwrap();

    // send tx to ckb node
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of to address
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = GetBalancePayload {
        item: JsonItem::Address(to_address.to_string()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();

    assert_eq!(to_balance.balances.len(), 0);

    // get balance of to identity
    let to_identity = new_identity_from_secp_address(&to_address.to_string()).unwrap();
    let payload_to = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload_to.clone()).unwrap();
    let (ckb_balance, udt_balance) =
        if to_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&to_balance.balances[0], &to_balance.balances[1])
        } else {
            (&to_balance.balances[1], &to_balance.balances[0])
        };

    assert_eq!(to_balance.balances.len(), 2);
    assert_eq!(ckb_balance.free, 0u64.to_string());
    assert_eq!(ckb_balance.occupied, 162_0000_0000u64.to_string());
    assert_eq!(udt_balance.claimable, 100u64.to_string());

    // get balance of owner identity
    let owner_identity = new_identity_from_secp_address(&owner_address.to_string()).unwrap();
    let payload_owner = GetBalancePayload {
        item: JsonItem::Identity(owner_identity.encode()),
        asset_infos: HashSet::new(),
        tip_block_number: None,
    };
    let owner_balance = mercury_client.get_balance(payload_owner.clone()).unwrap();
    let owner_left_capacity = owner_balance.balances[0].free.parse::<u64>().unwrap();

    assert_eq!(owner_balance.balances.len(), 1);
    assert!(owner_left_capacity < 88_0000_0000);
    assert!(owner_left_capacity > 87_0000_0000);

    // after 6 epoch
    fast_forward_epochs(CHEQUE_LOCK_EPOCH as usize).unwrap();

    // get balance of to identity
    let to_balance = mercury_client.get_balance(payload_to).unwrap();
    assert_eq!(to_balance.balances.len(), 0);

    // get balance of owner identity
    let owner_balance = mercury_client.get_balance(payload_owner).unwrap();
    let (ckb_balance, udt_balance) =
        if owner_balance.balances[0].asset_info.asset_type == AssetType::CKB {
            (&owner_balance.balances[0], &owner_balance.balances[1])
        } else {
            (&owner_balance.balances[1], &owner_balance.balances[0])
        };

    assert_eq!(owner_balance.balances.len(), 2);
    assert_eq!(ckb_balance.occupied, 162_0000_0000u64.to_string());
    assert_eq!(udt_balance.free, 100u64.to_string());
}

inventory::submit!(IntegrationTest {
    name: "test_issue_udt_hold_by_from",
    test_fn: test_issue_udt_hold_by_from
});
