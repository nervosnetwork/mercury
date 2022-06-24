use super::*;

use ckb_types::core::ScriptHashType;
use ckb_types::{packed, H256};
use std::str::FromStr;

#[tokio::test]
async fn test_get_historical_live_cells_desc() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash_1: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "57ccb07be6875f61d93636b0ee11b675494627d2",
        ScriptHashType::Type,
    );
    let lock_hash_2: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "3f1573b44218d4c12a91919a58a863be415a2bc3",
        ScriptHashType::Type,
    );
    let lock_hash_3: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "64257f00b6b63e987609fa9be2d0c86d351020fb",
        ScriptHashType::Type,
    );

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest::default(),
        )
        .await
        .unwrap()
        .response
        .into_iter();
    assert_eq!(3, ret.len());

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: Some(u16::MAX),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(3, ret.response.len());

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: Some(1),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(1, ret.response.len());
    let index: u32 = ret.response[0].out_point.index().unpack();
    assert_eq!(9u32, index);
    assert_eq!(None, ret.count);

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: ret.next_cursor,
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    let index: u32 = ret.response[1].out_point.index().unpack();
    assert_eq!(7u32, index);
    assert_eq!(2, ret.response.len());

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![lock_hash_1, lock_hash_2, lock_hash_3],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: ret.next_cursor,
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(0, ret.response.len());
}

#[tokio::test]
async fn test_get_historical_live_cells_asc() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash_1: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "57ccb07be6875f61d93636b0ee11b675494627d2",
        ScriptHashType::Type,
    );
    let lock_hash_2: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "3f1573b44218d4c12a91919a58a863be415a2bc3",
        ScriptHashType::Type,
    );
    let lock_hash_3: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "64257f00b6b63e987609fa9be2d0c86d351020fb",
        ScriptHashType::Type,
    );

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: Some(0),
                order: Order::Asc,
                limit: Some(1),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(1, ret.response.len());
    let index: u32 = ret.response[0].out_point.index().unpack();
    assert_eq!(7u32, index);
    assert_eq!(Some(3), ret.count);

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: ret.next_cursor,
                order: Order::Asc,
                limit: Some(2),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(2, ret.response.len());
    let index: u32 = ret.response[1].out_point.index().unpack();
    assert_eq!(9u32, index);

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![lock_hash_1, lock_hash_2, lock_hash_3],
            vec![],
            10,
            None,
            PaginationRequest {
                cursor: ret.next_cursor,
                order: Order::Asc,
                limit: Some(2),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(0, ret.response.len());
}

#[tokio::test]
async fn test_get_historical_live_cells_by_out_point() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash_1: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "57ccb07be6875f61d93636b0ee11b675494627d2",
        ScriptHashType::Type,
    );
    let lock_hash_2: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "3f1573b44218d4c12a91919a58a863be415a2bc3",
        ScriptHashType::Type,
    );
    let lock_hash_3: H256 = caculate_lock_hash(
        "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "64257f00b6b63e987609fa9be2d0c86d351020fb",
        ScriptHashType::Type,
    );

    let tx_hash =
        H256::from_str("8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").unwrap();
    let out_point = packed::OutPointBuilder::default()
        .tx_hash(tx_hash.pack())
        .index(7u32.pack())
        .build();

    let ret = pool
        .get_historical_live_cells(
            Context::new(),
            vec![
                lock_hash_1.clone(),
                lock_hash_2.clone(),
                lock_hash_3.clone(),
            ],
            vec![],
            10,
            Some(out_point),
            PaginationRequest {
                cursor: Some(0),
                order: Order::Asc,
                limit: Some(100),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(1, ret.response.len());
    let index: u32 = ret.response[0].out_point.index().unpack();
    assert_eq!(7u32, index);
}
