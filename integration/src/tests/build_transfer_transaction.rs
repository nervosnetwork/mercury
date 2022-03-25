use super::IntegrationTest;
use crate::utils::instruction::prepare_address_with_ckb_capacity;

fn test_prepare_address() {
    let _address_with_200_ckb =
        prepare_address_with_ckb_capacity(200_0000_0000).expect("prepare ckb");
}

inventory::submit!(IntegrationTest {
    name: "test_prepare_address",
    test_fn: test_prepare_address
});
