#![allow(
    clippy::from_over_into,
    clippy::mutable_key_type,
    clippy::upper_case_acronyms
)]

mod error;
mod extensions;
mod rpc;
mod service;
mod stores;
mod types;

use crate::service::Service;
use crate::types::{ExtensionsConfig, JsonExtensionsConfig};

use clap::{crate_version, App, Arg};
use jsonrpc_core_client::transports::http;
use log::debug;

use std::fs::read_to_string;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .init();
    let matches = App::new("ckb-companion")
        .version(crate_version!())
        .arg(
            Arg::with_name("ckb_uri")
                .short("c")
                .help("CKB rpc http service uri, default http://127.0.0.1:8114")
                .default_value("http://127.0.0.1:8114")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("listen_uri")
                .short("l")
                .help("Indexer rpc http service listen address, default 127.0.0.1:8116")
                .default_value("127.0.0.1:8116")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("store_path")
                .short("s")
                .help("Sets the indexer store path to use")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config_path")
                .short("p")
                .help("Path of extension configs to load")
                .takes_value(true),
        )
        .get_matches();

    let extensions_config = match matches.value_of("config_path") {
        Some(path) => {
            let config_data = read_to_string(&path).expect("Cannot read config file!");
            let config: JsonExtensionsConfig =
                serde_json::from_str(&config_data).expect("Cannot parse config!");
            config.into()
        }
        None => ExtensionsConfig::default(),
    };

    let service = Service::new(
        matches.value_of("store_path").expect("required arg"),
        matches.value_of("listen_uri").expect("required uri"),
        std::time::Duration::from_secs(2),
        extensions_config,
    )
    .expect("Service creating failure!");
    let rpc_server = service.start();
    debug!("Running!");

    let mut uri = matches
        .value_of("ckb_uri")
        .expect("require ckb uri")
        .to_owned();
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
