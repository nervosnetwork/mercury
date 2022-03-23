use super::IntegrationTest;
use crate::utils::prepare_address_with_ckb_capacity;

fn test_prepare_address() {
    let address = prepare_address_with_ckb_capacity(10000000000);
    println!("{:?}", address);
}

inventory::submit!(IntegrationTest {
    name: "test_prepare_address",
    test_fn: test_prepare_address
});
