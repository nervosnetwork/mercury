use super::IntegrationTest;
use crate::utils::instruction::generate_blocks;

fn test_generate_blocks() {
    assert!(generate_blocks(1).is_ok());
}

inventory::submit!(IntegrationTest {
    name: "test_generate_blocks",
    test_fn: test_generate_blocks
});
