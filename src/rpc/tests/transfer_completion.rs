use super::*;

use crate::rpc::types::{Action, FromAccount, Source, ToAccount, TransferItem, TransferPayload};

#[test]
fn test_ckb_transfer_complete() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0),
        AddressData::new(addr_2, 0, 200),
        AddressData::new(addr_3, 500_000, 0),
    ]);

    let payload = TransferPayload {
        udt_hash: None,
        fee: 100 * BYTE_SHANNONS,
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
            amount: 100 * BYTE_SHANNONS as u128,
        }],
    };

    let rpc = engine.rpc();
    let ret = rpc.transfer_completion(payload).unwrap();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
}

#[test]
fn test() {
    let addr_1 = "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70";
    let addr_2 = "ckt1qyq2y6jdkynen2vx946tnsdw2dgucvv7ph0s8n4kfd";
    let addr_3 = "ckt1qyq98qe26z8eg8q0852h622m40s50swtqnrqndruht";

    let engine = RpcTestEngine::init_data(vec![
        AddressData::new(addr_1, 500_000, 0),
        AddressData::new(addr_2, 0, 200),
        AddressData::new(addr_3, 500_000, 0),
    ]);

    let rpc = engine.rpc();
    let ret = rpc.get_ckb_balance(addr_3.to_string()).unwrap();

    write_file(serde_json::to_string_pretty(&ret).unwrap());
}
