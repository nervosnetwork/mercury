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
use jsonrpc_core_client::transports::http;
use log::{info, LevelFilter};
use log4rs::append::{console::ConsoleAppender, file::FileAppender};
use log4rs::config::{Appender, Root};
use log4rs::{encode::pattern::PatternEncoder, Config};
use tokio_compat::FutureExt;

use std::str::FromStr;

const CONSOLE: &str = "console";

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

    log_init(
        mercury_config.log_path.as_str(),
        LevelFilter::from_str(&mercury_config.log_level).unwrap(),
    );

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

fn log_init(log_path: &str, log_level: LevelFilter) {
    let mut root_builder = Root::builder();

    if log_path == CONSOLE {
        root_builder = root_builder.appender("console");
    } else {
        root_builder = root_builder.appender("file")
    }

    let root = root_builder.build(log_level);
    let encoder = Box::new(PatternEncoder::new("[{d} {h({l})} {t}] {m}{n}"));

    let config = if log_path == CONSOLE {
        let console_appender = ConsoleAppender::builder().encoder(encoder).build();
        Config::builder()
            .appender(Appender::builder().build("console", Box::new(console_appender)))
            .build(root)
    } else {
        let file_appender = FileAppender::builder()
            .encoder(encoder)
            .build(log_path)
            .expect("build file logger");
        Config::builder()
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(root)
    };

    log4rs::init_config(config.expect("build log config")).unwrap();
}
