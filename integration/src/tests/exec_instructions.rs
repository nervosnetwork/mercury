use super::IntegrationTest;
use crate::utils::instruction::generate_block;

fn test_generate_block() {
    assert!(generate_block().is_ok())
}

inventory::submit!(IntegrationTest {
    name: "test_generate_block",
    test_fn: test_generate_block
});
