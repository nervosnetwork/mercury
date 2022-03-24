pub mod tests;
pub mod utils;

use tests::IntegrationTest;
use utils::client::{handle_response, RpcClient};
use utils::const_definition::{
    CELL_BASE_MATURE_EPOCH, CKB_URI, MERCURY_URI, RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS,
};
use utils::mercury_types::SyncState;

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
    for _try in 0..=RPC_TRY_COUNT {
        let resp = utils::try_post_http_request(
            CKB_URI,
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "local_node_info"
            }"#,
        );
        if resp.is_ok() {
            unlock_frozen_capacity_in_genesis();
            return ckb;
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
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
    for _try in 0..=RPC_TRY_COUNT {
        let resp = utils::try_post_http_request(
            MERCURY_URI,
            r#"{
                "id": 42,
                "jsonrpc": "2.0",
                "method": "get_mercury_info"
            }"#,
        );
        if resp.is_ok() {
            loop {
                let mercury_client = RpcClient::new(MERCURY_URI.to_string());
                let request = mercury_client
                    .build_request("get_sync_state".to_string(), ())
                    .expect("get sync state");
                let response = mercury_client
                    .rpc_exec(&request)
                    .expect("exec rpc sync state");
                let sync_state: SyncState =
                    handle_response(response).expect("handle response of sync state");
                if let SyncState::Serial(progress) = sync_state {
                    println!("{:?}", progress);
                    if progress.current == progress.target {
                        break;
                    }
                }
                sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
            }
            return (ckb, mercury);
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb, mercury]);
    panic!("Setup test environment failed");
}

fn unlock_frozen_capacity_in_genesis() {
    let resp = utils::post_http_request(
        CKB_URI,
        r#"{
            "id": 42,
            "jsonrpc": "2.0",
            "method": "get_current_epoch"
        }"#,
    );
    let current_epoch_number = &resp["result"]["number"]
        .as_str()
        .expect("get current epoch number")
        .trim_start_matches("0x");
    let current_epoch_number = u64::from_str_radix(current_epoch_number, 16).unwrap();
    if current_epoch_number < CELL_BASE_MATURE_EPOCH {
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number + 1) * 1000 {
            let ckb_client = RpcClient::new(CKB_URI.to_string());
            let request = ckb_client
                .build_request("generate_block".to_string(), ())
                .expect("build request");
            let response = ckb_client.rpc_exec(&request);
            println!("{:?}", response);
        }
    }
}
