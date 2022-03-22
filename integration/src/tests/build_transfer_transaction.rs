use super::IntegrationTest;
use crate::utils::generate_rand_secp_address_pk_pair;

fn test_transfer_ckb_hold_by_from() {
    let (_address, _pk) = generate_rand_secp_address_pk_pair();
    println!("");
}

inventory::submit!(IntegrationTest {
    name: "test_transfer_ckb_hold_by_from",
    test_fn: test_transfer_ckb_hold_by_from
});
