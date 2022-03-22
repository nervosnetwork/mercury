pub mod tests;
pub mod utils;

use tests::IntegrationTest;

use std::panic;
use std::process::Child;
use std::thread::sleep;
use std::time::{Duration, Instant};

const TRY_COUNT: usize = 10;
const TRY_INTERVAL_SECS: u64 = 5;

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
    let (ckb, mercury) = start_mercury(ckb);
    vec![ckb, mercury]
}

fn teardown(childs: Vec<Child>) {
    for mut child in childs {
        child.kill().expect("teardown failed");
    }
}

fn start_ckb_node() -> Child {
    let ckb = utils::run(
        "ckb",
        vec!["run", "-C", "dev_chain/dev", "--skip-spec-check"],
    )
    .expect("start ckb dev chain");
    for _try in 0..=TRY_COUNT {
        let resp = utils::try_post_http_request(
            "http://127.0.0.1:8114".to_string(),
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "local_node_info"
            }"#,
        );
        if resp.is_ok() {
            return ckb;
        } else {
            sleep(Duration::from_secs(TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb]);
    panic!("Setup test environment failed");
}

fn start_mercury(ckb: Child) -> (Child, Child) {
    let mercury = utils::run(
        "cargo",
        vec![
            "run",
            "--manifest-path",
            "../Cargo.toml",
            "--",
            "-c",
            "dev_chain/devnet_config.toml",
            "run",
        ],
    )
    .expect("start ckb dev chain");
    for _try in 0..=TRY_COUNT {
        let resp = utils::try_post_http_request(
            "http://127.0.0.1:8116".to_string(),
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "get_mercury_info"
            }"#,
        );
        if resp.is_ok() {
            return (ckb, mercury);
        } else {
            sleep(Duration::from_secs(TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb, mercury]);
    panic!("Setup test environment failed");
}
