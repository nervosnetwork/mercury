use super::*;

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
    let ret = rpc
        .get_cells_by_lock_script(&(parse_address(addr_1).unwrap().payload().into()))
        .unwrap();
    println!("{:?}", ret);
}
