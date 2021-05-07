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
mod utils;

use crate::config::{parse, MercuryConfig};
use crate::service::Service;

use clap::{crate_version, App, Arg};
use env_logger::fmt::TimestampPrecision::Millis;
use jsonrpc_core_client::transports::http;
use log::{info, LevelFilter};
use tokio_compat::FutureExt;

use std::str::FromStr;

#[tokio::main]
async fn main() {
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

    env_logger::Builder::from_default_env()
        .format_timestamp(Some(Millis))
        .filter_level(LevelFilter::from_str(&mercury_config.log_level).unwrap())
        .init();

    let service = Service::new(
        mercury_config.store_path.as_str(),
        mercury_config.listen_uri.as_str(),
        std::time::Duration::from_secs(2),
        mercury_config.network_type.as_str(),
        mercury_config.to_json_extensions_config().into(),
    );

    let rpc_server = service.start();
    info!("Running!");

    let mut uri = mercury_config.ckb_uri.clone();
    if !uri.starts_with("http") {
        uri = format!("http://{}", uri);
    }

    let client = http::connect(&uri)
        .compat()
        .await
        .unwrap_or_else(|_| panic!("Failed to connect to {:?}", uri));

    service.poll(client).await;

    rpc_server.close();
    info!("Closing!");
}
