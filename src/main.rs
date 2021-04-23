#![allow(
    clippy::from_over_into,
    clippy::mutable_key_type,
    clippy::upper_case_acronyms
)]

mod config;
mod error;
mod extensions;
mod rpc;
mod service;
mod stores;
mod types;

use crate::config::{parse, MercuryConfig};
use crate::service::Service;

use clap::{crate_version, App, Arg};
use jsonrpc_core_client::transports::http;
use log::debug;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .init();
    let matches = App::new("mercury")
        .version(crate_version!())
        .arg(
            Arg::with_name("config_path")
                .short("c")
                .help("Mercury config path")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let mercury_config: MercuryConfig =
        parse(matches.value_of("config_path").expect("missing config")).unwrap();

    let service = Service::new(
        mercury_config.ckb_uri.as_str(),
        mercury_config.listen_uri.as_str(),
        std::time::Duration::from_secs(2),
        mercury_config.to_json_extensions_config().into(),
    )
    .expect("Service creating failure!");

    let rpc_server = service.start();
    debug!("Running!");

    let mut uri = mercury_config.ckb_uri.clone();
    if !uri.starts_with("http") {
        uri = format!("http://{}", uri);
    }

    let client = http::connect(&uri)
        .await
        .unwrap_or_else(|_| panic!("Failed to connect to {:?}", uri));

    service.poll(client).await;

    rpc_server.close();
    debug!("Closing!");
}
