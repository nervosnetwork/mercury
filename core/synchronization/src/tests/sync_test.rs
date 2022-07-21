use super::*;

use crate::table::to_rb_bytes;

use ckb_types::bytes::Bytes;

#[tokio::test]
async fn test_create_tables() {
    let _pool = connect_and_create_tables().await;
}

#[tokio::test]
async fn test_to_rb_bytes() {
    let tx_hash = hex::decode("63000000000000000000000000000000").unwrap();
    let ret_rbatis_bytes = to_rb_bytes(&tx_hash);
    let ret_bytes = Bytes::from(tx_hash);
    assert_eq!(ret_rbatis_bytes.len(), ret_bytes.len());
}
