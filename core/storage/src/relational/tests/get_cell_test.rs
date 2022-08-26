use super::*;

use std::str::FromStr;

#[tokio::test]
async fn test_get_cells_pagination_return_count() {
    let pool = connect_and_insert_blocks().await;

    let lock_hash =
        H256::from_str("ba93972fbe398074f4e0bc538d7e36e61a8b140585b52deb4d2890e8d9d320f0").unwrap();

    let cells = pool
        .get_cells(
            None,
            vec![lock_hash.clone()],
            vec![],
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
    assert_eq!(None, cells.count);

    let cells = pool
        .get_cells(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(Some(12), cells.count);
    assert_eq!(2, cells.response.len());
}

#[tokio::test]
async fn test_is_not_live_cell() {
    let storage = connect_and_insert_blocks().await;
    let tx_hash =
        hex::decode("8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").unwrap();
    let query = sqlx::query(
        "SELECT id FROM mercury_live_cell 
        WHERE tx_hash = $1 AND output_index = $2",
    )
    .bind(tx_hash)
    .bind(5);
    let pool = storage.sqlx_pool.get_pool().unwrap();
    let res = query.fetch_optional(pool).await.unwrap();
    assert!(res.is_none());
}

#[tokio::test]
async fn test_is_live_cell() {
    let storage = connect_and_insert_blocks().await;
    let tx_hash =
        hex::decode("8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").unwrap();
    let query = sqlx::query(
        "SELECT id FROM mercury_live_cell 
            WHERE tx_hash = $1 AND output_index = $2",
    )
    .bind(tx_hash)
    .bind(0);
    let pool = storage.sqlx_pool.get_pool().unwrap();
    let res = query.fetch_optional(pool).await.unwrap();
    assert!(res.is_some());
}

#[tokio::test]
async fn test_get_cells_out_point() {
    let pool = connect_and_insert_blocks().await;

    let tx_hash =
        h256!("0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f").pack();
    let out_point = packed::OutPoint::new(tx_hash, 5);
    let cells = pool
        .get_cells(
            Some(out_point),
            vec![],
            vec![],
            None,
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: None,
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(Some(1), cells.count);
    assert_eq!(Some(0), cells.response[0].consumed_block_number);
    assert_eq!(Some(1), cells.response[0].consumed_tx_index);

    let tx_hash =
        h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37").pack();
    let out_point = packed::OutPoint::new(tx_hash, 1);
    let cells = pool
        .get_cells(
            Some(out_point),
            vec![],
            vec![],
            None,
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: None,
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(Some(1), cells.count);
    assert_eq!(None, cells.response[0].consumed_tx_index);
    assert_eq!(None, cells.response[0].consumed_block_number);
}

#[tokio::test]
async fn test_get_cells_pagination_cursor() {
    let pool = connect_and_insert_blocks().await;

    let cells = pool
        .get_cells(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: None,
                order: Order::Asc,
                limit: Some(2),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    let index_0: u32 = cells.response[0].out_point.index().unpack();
    let index_1: u32 = cells.response[1].out_point.index().unpack();

    assert_eq!(Some(12), cells.count);
    assert_eq!(2, cells.response.len());
    assert_eq!(0, index_0);
    assert_eq!(1, index_1);

    let cells = pool
        .get_cells(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: cells.next_cursor,
                order: Order::Asc,
                limit: Some(2),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    let index_2: u32 = cells.response[0].out_point.index().unpack();
    let index_3: u32 = cells.response[1].out_point.index().unpack();

    assert_eq!(Some(12), cells.count);
    assert_eq!(2, cells.response.len());
    assert_eq!(2, index_2);
    assert_eq!(3, index_3);
}

#[tokio::test]
async fn test_get_cells_pagination_skip() {
    let pool = connect_and_insert_blocks().await;

    let cells = pool
        .get_cells(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: None,
                order: Order::Asc,
                limit: Some(2),
                skip: Some(4),
                return_count: false,
            },
        )
        .await
        .unwrap();
    let index_4: u32 = cells.response[0].out_point.index().unpack();
    let index_5: u32 = cells.response[1].out_point.index().unpack();

    assert_eq!(None, cells.count);
    assert_eq!(2, cells.response.len());
    assert_eq!(4, index_4);
    assert_eq!(5, index_5);

    let cells = pool
        .get_cells(
            None,
            vec![],
            vec![],
            Some(Range::new(0, 9)),
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: Some(2),
                skip: Some(4),
                return_count: false,
            },
        )
        .await
        .unwrap();
    let index_7: u32 = cells.response[0].out_point.index().unpack();
    let index_6: u32 = cells.response[1].out_point.index().unpack();

    assert_eq!(None, cells.count);
    assert_eq!(2, cells.response.len());
    assert_eq!(7, index_7);
    assert_eq!(6, index_6);
}
