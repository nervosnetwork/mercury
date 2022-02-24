pub mod tests;
use tests::IntegrationTest;

fn main() {
    // Setup test environment
    setup();

    // Run all tests
    for t in inventory::iter::<IntegrationTest> {
        (t.test_fn)()
    }

    // Run the test
    let t = IntegrationTest::from_name("test_get_balance").unwrap();
    (t.test_fn)();

    // Teardown test environment
    teardown();
}

fn setup() {
    println!("Setup");
    start_ckb_node();
}

fn teardown() {
    println!("Teardown")
}

fn start_ckb_node() {}
