#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod error;
mod extensions;
mod service;
mod types;

use crate::{
    service::Service,
    types::{ExtensionsConfig, JsonExtensionsConfig},
};
use clap::{crate_version, App, Arg};
use futures::Future;
use hyper::rt;
use jsonrpc_core_client::transports::http;
use std::fs::read_to_string;

fn main() {
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

    rt::run(rt::lazy(move || {
        let mut uri = matches
            .value_of("ckb_uri")
            .expect("require ckb uri")
            .to_owned();
        if !uri.starts_with("http") {
            uri = format!("http://{}", uri);
        }

        http::connect(&uri)
            .and_then(move |client| {
                service.poll(client);
                Ok(())
            })
            .map_err(|e| {
                println!("Error: {:?}", e);
            })
    }));

    rpc_server.close();
    debug!("Closing!");
}
