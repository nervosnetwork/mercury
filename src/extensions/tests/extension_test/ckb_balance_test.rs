use super::*;

use ckb_chain_spec::consensus::Consensus;

#[test]
fn test_append_genesis() {
    let mut test = TestHandler::new(ExtensionsConfig::default());
    let (ext, batch_store) = test.ckb_balance_extension();

    let genesis = Consensus::default().genesis_block;

    ext.append(&genesis).unwrap();

    batch_store.commit().unwrap();

    assert_eq!(
        ext.get_balance(&*GENESIS_OUTPUT_ADDRESS.to_string())
            .unwrap()
            .unwrap(),
        *GENESIS_CAPACITY
    );
}
