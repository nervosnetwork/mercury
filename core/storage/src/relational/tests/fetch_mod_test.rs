use super::*;

#[tokio::test]
async fn test_query_live_cells() {
    let pool = connect_and_insert_blocks().await;

    let ret = pool
        .query_live_cells(
            Context::new(),
            None,
            vec![],
            vec![],
            Some(Range::new(0, 1)),
            None,
            None,
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(2, ret.response.len());
    assert_eq!(Some(11), ret.count);

    let ret = pool
        .query_live_cells(
            Context::new(),
            None,
            vec![],
            vec![],
            Some(Range::new(0, 1)),
            None,
            None,
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: Some(2),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(2, ret.response.len());
    assert_eq!(None, ret.count);
}

#[tokio::test]
async fn test_query_indexer_cells() {
    let pool = connect_and_insert_blocks().await;

    let ret = pool
        .query_indexer_cells(
            vec![],
            vec![],
            Some(Range::new(0, 1)),
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
    assert_eq!(2, ret.response.len());
    assert_eq!(Some(13), ret.count);

    let ret = pool
        .query_indexer_cells(
            vec![],
            vec![],
            Some(Range::new(0, 10)),
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: Some(3),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(3, ret.response.len());
    assert_eq!(None, ret.count);
}

#[tokio::test]
async fn test_query_transactions() {
    let pool = connect_and_insert_blocks().await;

    let ret = pool
        .query_transactions_(
            Context::new(),
            vec![],
            Some(Range::new(0, 2)),
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
    assert_eq!(2, ret.response.len());
    assert_eq!(Some(4), ret.count);

    let ret = pool
        .query_transactions_(
            Context::new(),
            vec![],
            Some(Range::new(0, 2)),
            PaginationRequest {
                cursor: Some(u64::MAX),
                order: Order::Desc,
                limit: Some(3),
                skip: None,
                return_count: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(3, ret.response.len());
    assert_eq!(None, ret.count);
}
