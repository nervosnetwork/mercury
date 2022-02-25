pub mod tests;
pub mod utils;

use anyhow::Result;
use tests::IntegrationTest;

use std::process::Child;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // Setup test environment
    println!("Setup");
    let child_ckb = start_ckb_node().unwrap();
    sleep(Duration::from_secs(5));
    let child_mercury = start_mercury().unwrap();

    sleep(Duration::from_secs(5));

    // Run all tests
    for t in inventory::iter::<IntegrationTest> {
        (t.test_fn)()
    }

    // Run the test
    let t = IntegrationTest::from_name("test_get_balance").unwrap();
    (t.test_fn)();

    // Teardown test environment
    teardown(child_mercury);
    teardown(child_ckb);
}

fn teardown(mut child: Child) {
    println!("Teardown");
    child.kill().expect("msg");
}

fn start_ckb_node() -> Result<Child> {
    let child = utils::run(
        "ckb",
        vec!["run", "-C", "free-space/ckb-dev", "--skip-spec-check"],
    )
    .expect("start ckb dev chain");
    Ok(child)
}

fn start_mercury() -> Result<Child> {
    let child = utils::run(
        "cargo",
        vec![
            "run",
            "--bin",
            "mercury",
            "--",
            "-c",
            "devtools/config/devnet_config.toml",
            "run",
        ],
    )
    .expect("start ckb dev chain");
    Ok(child)
}
