#![allow(
    clippy::from_over_into,
    clippy::mutable_key_type,
    clippy::upper_case_acronyms
)]

mod cli;
mod config;
mod error;
mod extensions;
mod rpc;
mod service;
mod stores;
mod types;
mod utils;

use tokio_compat::FutureExt;

#[tokio::main]
async fn main() {
    let mercury = cli::Cli::init();
    mercury.start().compat().await;
}
