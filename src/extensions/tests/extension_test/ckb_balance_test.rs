use super::*;

use ckb_chain_spec::consensus::Consensus;

#[test]
fn test_append_genesis() {
    let mut test = TestHandler::new(ExtensionsConfig::default());
    let ext = test.ckb_balance_extension();

    let genesis = Consensus::default().genesis_block;

    ext.append(&genesis).unwrap();

    assert_eq!(
        ext.get_balance(&*GENESIS_LOCK_ARGS).unwrap().unwrap(),
        *GENESIS_CAPACITY
    );
}
