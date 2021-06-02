use super::*;

use crate::rpc::types::{Action, FromAccount, Source, ToAccount, TransferItem, TransferPayload};

fn response_assert(
    response: &TransferCompletionResponse,
    expected_output_len: usize,
    expected_sigs_len: usize,
) {
    assert_eq!(response.tx_view.inner.outputs.len(), expected_output_len);
    assert_eq!(response.sigs_entry.len(), expected_sigs_len);
}

#[test]
fn test_ckb_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0),
        AddressData::new(addr_2, 0, 200),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100,
        change: None,
        from: FromAccount {
            idents: vec![addr_1.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_2.to_string(),
                action: Action::PayByFrom,
            },
            amount: 100u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.transfer_completion(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 2, 1);

    assert_eq!(
        ret.sigs_entry[0].pub_key.as_bytes(),
        parse_address(addr_1).unwrap().payload().args()
    );
    assert_eq!(tx_outputs[0].capacity, (100 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (499_800 * BYTE_SHANNONS).into());
}

#[test]
fn test_udt_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    //let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0),
        AddressData::new(addr_2, 400, 10_000),
        //AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: Some(SUDT_HASH.read().clone()),
        fee: 5,
        change: None,
        from: FromAccount {
            idents: vec![addr_2.to_string()],
            source: Source::Owned,
        },
        items: vec![TransferItem {
            to: ToAccount {
                ident: addr_1.to_string(),
                action: Action::PayByFrom,
            },
            amount: 100u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.transfer_completion(payload).unwrap();
    let tx_outputs = ret.tx_view.inner.outputs.clone();
    let tx_data = ret.tx_view.inner.outputs_data.clone();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
    response_assert(&ret, 3, 1);

    assert_eq!(
        ret.sigs_entry[0].pub_key.as_bytes(),
        parse_address(addr_2).unwrap().payload().args()
    );
    assert_eq!(tx_outputs[0].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(tx_outputs[1].capacity, (142 * BYTE_SHANNONS).into());
    assert_eq!(
        tx_outputs[2].capacity,
        ((400 - 142 - 5) * BYTE_SHANNONS).into()
    );
    assert_eq!(decode_udt_amount(tx_data[0].as_bytes()), 100);
    assert_eq!(decode_udt_amount(tx_data[1].as_bytes()), (10_000 - 100));
}
