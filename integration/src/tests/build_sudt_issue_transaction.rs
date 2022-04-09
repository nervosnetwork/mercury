use super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::{
    generate_rand_secp_address_pk_pair, get_udt_hash_by_owner, new_identity_from_secp_address,
};
use crate::utils::instruction::{issue_udt_with_cheque, prepare_address_with_ckb_capacity};
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{AssetInfo, GetBalancePayload, JsonItem};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_issue_udt_hold_by_from",
    test_fn: test_issue_udt_hold_by_from
});
fn test_issue_udt_hold_by_from() {
    // prepare
    let (owner_address, owner_pk) =
        prepare_address_with_ckb_capacity(250_0000_0000).expect("prepare ckb");
    let (to_address, _to_pk) = generate_rand_secp_address_pk_pair();

    // issue udt
    let _tx_hash = issue_udt_with_cheque(&owner_address, &owner_pk, &to_address, 100u64);

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // get balance of to identity, AssetType::UDT
    let to_identity = new_identity_from_secp_address(&to_address.to_string()).unwrap();
    let udt_hash = get_udt_hash_by_owner(&owner_address).unwrap();
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(udt_hash));
    let payload_to = GetBalancePayload {
        item: JsonItem::Identity(to_identity.encode()),
        asset_infos,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload_to).unwrap();
    let udt_balance = &to_balance.balances[0];

    assert_eq!(to_balance.balances.len(), 1);
    assert_eq!(udt_balance.free, 100u64.to_string());
}
