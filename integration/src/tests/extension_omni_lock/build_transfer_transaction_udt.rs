use super::super::IntegrationTest;
use crate::const_definition::{
    MERCURY_URI, UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_ACP_ADDRESS_PK,
};
use crate::utils::address::{
    acp::build_acp_address, generate_rand_secp_address_pk_pair, new_identity_from_secp_address,
};
use crate::utils::instruction::{issue_udt_1, send_transaction_to_ckb};
use crate::utils::rpc_client::MercuryRpcClient;
use crate::utils::signer::sign_transaction;

use core_rpc_types::{
    AssetInfo, GetBalancePayload, JsonItem, OutputCapacityProvider, ToInfo, TransferPayload,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_omni_transfer_udt_from_provide_capacity_acp",
    test_fn: test_omni_transfer_udt_from_provide_capacity_acp
});
fn test_omni_transfer_udt_from_provide_capacity_acp() {
    // prepare udt
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let acp_address_with_udt = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let acp_address_pk = UDT_1_HOLDER_ACP_ADDRESS_PK.get().unwrap();

    // prepare to address
    let (to_address_secp, _to_address_pk) = generate_rand_secp_address_pk_pair();
    let to_acp_address = build_acp_address(&to_address_secp).unwrap();

    // transfer cheque udt from receiver
    let from_identity = new_identity_from_secp_address(&acp_address_with_udt.to_string()).unwrap();
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![JsonItem::Identity(hex::encode(from_identity.0))],
        to: vec![ToInfo {
            address: to_acp_address.to_string(),
            amount: 80u128.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(tx, &[acp_address_pk.to_owned()]).unwrap();
    let _tx_hash = send_transaction_to_ckb(tx).unwrap();

    // get balance of to address
    let to_identity = new_identity_from_secp_address(&to_address_secp.to_string()).unwrap();
    let asset_infos = HashSet::new();
    let payload = GetBalancePayload {
        item: JsonItem::Identity(hex::encode(to_identity.0)),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let to_balance = mercury_client.get_balance(payload).unwrap();
    let (ckb_balance, udt_balance) = (&to_balance.balances[0], &to_balance.balances[1]);

    assert_eq!(to_balance.balances.len(), 2);
    assert_eq!(ckb_balance.free, 0u128.into());
    assert_eq!(ckb_balance.occupied, 142_0000_0000u128.into());
    assert_eq!(udt_balance.free, 80u128.into());
}
