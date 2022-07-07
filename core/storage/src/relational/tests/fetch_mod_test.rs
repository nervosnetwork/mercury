use super::*;

#[tokio::test]
async fn test_query_live_cells() {
    let pool = connect_and_insert_blocks().await;

    let ret = pool
        .query_live_cells(
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
        .query_indexer_transactions(
            vec![],
            vec![],
            Some(Range::new(0, 1)),
            PaginationRequest {
                cursor: None,
                order: Order::Desc,
                limit: None,
                skip: None,
                return_count: true,
            },
        )
        .await
        .unwrap();
    let txs_input: Vec<_> = ret
        .response
        .iter()
        .filter(|tx| tx.io_type == IOType::Input)
        .collect();
    let txs_output: Vec<_> = ret
        .response
        .iter()
        .filter(|tx| tx.io_type == IOType::Output)
        .collect();
    assert_eq!(Some(13), ret.count);
    assert_eq!(1, txs_input.len());
    assert_eq!(12, txs_output.len());

    let ret = pool
        .query_indexer_transactions(
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
        .query_transactions(
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
        .query_transactions(
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

#[tokio::test]
async fn test_get_scripts() {
    use ckb_types::bytes::Bytes;
    use common::address::caculate_script_hash;
    use common::hash::blake2b_256_to_160;

    let code_hash = "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8";
    let args = "3f1573b44218d4c12a91919a58a863be415a2bc3";
    let script_hash_type = ScriptHashType::Type;
    let script_hash = "8abf38905f28fd36088ebbbfdb021c2f4a853d2c9e8809338a381561a77bb241";

    let lock_hash = caculate_script_hash(code_hash, args, script_hash_type).unwrap();
    assert_eq!(script_hash, &lock_hash.to_string());

    let pool = connect_and_insert_blocks().await;
    let args = Bytes::from(hex::decode(args).unwrap());
    let script_hash = blake2b_256_to_160(&H256::from_str(script_hash).unwrap());
    let code_hash = H256::from_str(code_hash).unwrap();

    let ret = pool
        .get_scripts(
            Context::new(),
            vec![script_hash.clone()],
            vec![],
            None,
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(1, ret.len());

    let ret = pool
        .get_scripts(Context::new(), vec![], vec![], None, vec![args.clone()])
        .await
        .unwrap();
    assert_eq!(1, ret.len());

    let ret = pool
        .get_scripts(
            Context::new(),
            vec![script_hash],
            vec![code_hash],
            Some(args.len()),
            vec![args],
        )
        .await
        .unwrap();
    assert_eq!(1, ret.len());
}

#[tokio::test]
async fn test_get_scripts_by_partial_arg() {
    use ckb_types::bytes::Bytes;
    use common::address::caculate_script_hash;

    let code_hash = "9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8";
    let args = "3f1573b44218d4c12a91919a58a863be415a2bc3";
    let script_hash_type = ScriptHashType::Type;
    let script_hash = "8abf38905f28fd36088ebbbfdb021c2f4a853d2c9e8809338a381561a77bb241";

    let lock_hash = caculate_script_hash(code_hash, args, script_hash_type).unwrap();
    assert_eq!(script_hash, &lock_hash.to_string());

    let pool = connect_and_insert_blocks().await;
    let args = Bytes::from(hex::decode(args).unwrap());
    let script_hash = H256::from_str(script_hash).unwrap();
    let code_hash = H256::from_str(code_hash).unwrap();

    let ret = pool
        .get_scripts_by_partial_arg(Context::new(), &code_hash, args, (0, 20))
        .await
        .unwrap();
    assert_eq!(1, ret.len());
    assert_eq!(
        script_hash,
        bytes_to_h256(ret[0].calc_script_hash().as_slice())
    )
}
