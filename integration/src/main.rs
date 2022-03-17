pub mod tests;
pub mod utils;

use tests::IntegrationTest;

use std::panic;
use std::process::Child;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    // Setup test environment
    let child_handlers = setup();

    let (mut count_ok, mut count_failed) = (0, 0);
    let now = Instant::now();

    // Run all tests
    for t in inventory::iter::<IntegrationTest> {
        let result = panic::catch_unwind(|| {
            (t.test_fn)();
        });
        let flag = if result.is_ok() {
            count_ok += 1;
            "Ok"
        } else {
            count_failed += 1;
            "FAILED"
        };
        println!("{} ... {}", t.name, flag);
    }

    // Run the test
    let t = IntegrationTest::from_name("test_get_balance_udt").unwrap();
    let result = panic::catch_unwind(|| {
        (t.test_fn)();
    });
    let flag = if result.is_ok() {
        count_ok += 1;
        "Ok"
    } else {
        count_failed += 1;
        "FAILED"
    };
    println!("{} ... {}", t.name, flag);

    let elapsed = now.elapsed();

    // Teardown test environment
    teardown(child_handlers);

    // Display result
    println!();
    println!("running {} tests", count_ok + count_failed);
    println!(
        "test result: {}. {} passed; {} failed; finished in {}s",
        if count_failed > 0 { "FAILED" } else { "ok" },
        count_ok,
        count_failed,
        elapsed.as_secs_f32()
    );
}

fn setup() -> Vec<Child> {
    println!("Setup test environment...");
    let ckb = start_ckb_node();
    sleep(Duration::from_secs(5));
    let mercury = start_mercury();
    sleep(Duration::from_secs(20));
    vec![ckb, mercury]
}

fn start_ckb_node() -> Child {
    let child = utils::run(
        "ckb",
        vec![
            "run",
            "-C",
            "integration/dev_chain/dev",
            "--skip-spec-check",
        ],
    )
    .expect("start ckb dev chain");
    child
}

fn start_mercury() -> Child {
    let child = utils::run(
        "cargo",
        vec![
            "run",
            "--bin",
            "mercury",
            "--",
            "-c",
            "integration/dev_chain/devnet_config.toml",
            "run",
        ],
    )
    .expect("start ckb dev chain");
    child
}

fn teardown(childs: Vec<Child>) {
    for mut child in childs {
        child.kill().expect("teardown failed");
    }
}
