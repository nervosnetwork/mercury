use super::IntegrationTest;

fn test_get_balance() {
    println!("Running get_balance test")
}

fn test_get_balance_udt() {
    println!("Running get_balance_udt test")
}

inventory::submit!(IntegrationTest {
    name: "test_get_balance",
    test_fn: test_get_balance
});

inventory::submit!(IntegrationTest {
    name: "test_get_balance_udt",
    test_fn: test_get_balance_udt
});
