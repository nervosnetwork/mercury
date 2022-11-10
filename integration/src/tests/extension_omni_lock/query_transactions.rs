use super::super::IntegrationTest;
use crate::const_definition::MERCURY_URI;
use crate::utils::address::omni_lock::prepare_omni_ethereum_address_with_capacity;
use crate::utils::rpc_client::MercuryRpcClient;

use core_rpc_types::{
    AssetInfo, JsonItem, PaginationRequest, QueryTransactionsPayload, StructureType, TxView,
};

use std::collections::HashSet;

inventory::submit!(IntegrationTest {
    name: "test_omni_query_transactions_native",
    test_fn: test_omni_query_transactions_native
});
fn test_omni_query_transactions_native() {
    let (identity, _address, address_pk, out_point) =
        prepare_omni_ethereum_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let _pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = QueryTransactionsPayload {
        item: JsonItem::Identity(identity.encode()),
        asset_infos,
        extra: None,
        block_range: None,
        pagination: PaginationRequest::new(None, common::Order::Asc, None, true),
        structure_type: StructureType::Native,
    };
    let ret = mercury_client.query_transactions(payload).unwrap();
    assert_eq!(ret.count.unwrap(), 1.into());
    assert_eq!(ret.response.len(), 1);

    match &ret.response[0] {
        TxView::TransactionWithRichStatus(tx) => {
            assert_eq!(tx.tx_status.status, ckb_jsonrpc_types::Status::Committed);
            assert_eq!(tx.transaction.as_ref().unwrap().hash, out_point.tx_hash)
        }
        _ => panic!(),
    }
}

inventory::submit!(IntegrationTest {
    name: "test_omni_query_transactions_double_entry",
    test_fn: test_omni_query_transactions_double_entry
});
fn test_omni_query_transactions_double_entry() {
    let (identity, _address, address_pk, out_point) =
        prepare_omni_ethereum_address_with_capacity(300_0000_0000).expect("prepare ckb");
    let _pks = vec![address_pk];
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_ckb());
    let payload = QueryTransactionsPayload {
        item: JsonItem::Identity(identity.encode()),
        asset_infos,
        extra: None,
        block_range: None,
        pagination: PaginationRequest::new(None, common::Order::Asc, None, true),
        structure_type: StructureType::DoubleEntry,
    };
    let ret = mercury_client.query_transactions(payload).unwrap();
    assert_eq!(ret.count.unwrap(), 1.into());
    assert_eq!(ret.response.len(), 1);

    match &ret.response[0] {
        TxView::TransactionWithRichStatus(_) => panic!(),
        TxView::TransactionInfo(tx) => {
            let record = &tx.records[1];
            assert_eq!(tx.tx_hash, out_point.tx_hash);
            assert_eq!(tx.records.len(), 3);
            assert_eq!(record.out_point, out_point);
            assert_eq!(record.occupied, 0.into());
        }
    }
}
