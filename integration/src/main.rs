pub mod const_definition;
pub mod tests;
pub mod utils;

use clap::Parser;
use tests::IntegrationTest;
use utils::instruction::{setup, teardown};

use std::panic;
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the test case
    #[clap(short, long)]
    name: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Setup test environment
    let child_handlers = setup();

    let (mut count_ok, mut count_failed) = (0, 0);
    let now = Instant::now();

    match args.name.as_deref() {
        Some(name) => {
            // Run the test
            let t = IntegrationTest::from_name(name);
            if let Some(t) = t {
                let result = panic::catch_unwind(|| {
                    (t.test_fn)();
                });
                let flag = if result.is_ok() {
                    count_ok += 1;
                    "ok"
                } else {
                    count_failed += 1;
                    "FAILED"
                };
                println!("{} ... {}", t.name, flag);
            }
        }
        _ => {
            // Run all tests
            for t in inventory::iter::<IntegrationTest> {
                let result = panic::catch_unwind(|| {
                    (t.test_fn)();
                });
                let flag = if result.is_ok() {
                    count_ok += 1;
                    "ok"
                } else {
                    count_failed += 1;
                    "FAILED"
                };
                println!("{} ... {}", t.name, flag);
            }
        }
    }

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
