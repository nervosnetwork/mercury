use crate::config::{parse, MercuryConfig};
use crate::service::Service;

use clap::{crate_version, App, Arg, ArgMatches, SubCommand};
use jsonrpc_core_client::transports::http;
use log::{info, LevelFilter};
use log4rs::append::{console::ConsoleAppender, file::FileAppender};
use log4rs::config::{Appender, Root};
use log4rs::{encode::pattern::PatternEncoder, Config};

use std::str::FromStr;

const CONSOLE: &str = "console";

pub struct Cli<'a> {
    pub matches: ArgMatches<'a>,
    pub config: MercuryConfig,
}

impl<'a> Cli<'a> {
    pub fn init() -> Self {
        let matches = App::new("mercury")
            .version(crate_version!())
            .arg(
                Arg::with_name("config_path")
                    .short("c")
                    .long("config")
                    .help("Mercury config path")
                    .required(true)
                    .takes_value(true),
            )
            .subcommand(SubCommand::with_name("run").about("run the mercury process"))
            .subcommand(
                SubCommand::with_name("clear")
                    .about("clear some height data")
                    .arg(
                        Arg::with_name("height")
                            .short("h")
                            .long("height")
                            .help("clear the data after this height")
                            .required(true)
                            .takes_value(true),
                    ),
            )
            .get_matches();

        let config: MercuryConfig =
            parse(matches.value_of("config_path").expect("missing config")).unwrap();

        Cli { matches, config }
    }

    pub async fn start(&self) {
        self.log_init();

        match self.matches.subcommand() {
            ("run", None) => self.run().await,

            ("clear", Some(sub_cmd)) => self.clear(
                sub_cmd
                    .value_of("height")
                    .expect("missing clear start height")
                    .parse::<u64>()
                    .unwrap(),
            ),

            _ => self.run().await,
        }
    }

    async fn run(&self) {
        let service = Service::new(
            self.config.store_path.as_str(),
            self.config.listen_uri.as_str(),
            std::time::Duration::from_secs(2),
            self.config.network_type.as_str(),
            self.config.to_json_extensions_config().into(),
        );

        let rpc_server = service.start();
        info!("Running!");

        let mut uri = self.config.ckb_uri.clone();
        if !uri.starts_with("http") {
            uri = format!("http://{}", uri);
        }

        let client = http::connect(&uri)
            .await
            .unwrap_or_else(|_| panic!("Failed to connect to {:?}", uri));

        service.poll(client).await;

        rpc_server.close();
        info!("Closing!");
    }

    fn clear(&self, height: u64) {}

    fn log_init(&self) {
        let log_path = self.config.log_path.as_str();
        let mut root_builder = Root::builder();

        if log_path == CONSOLE {
            root_builder = root_builder.appender("console");
        } else {
            root_builder = root_builder.appender("file")
        }

        let root = root_builder.build(LevelFilter::from_str(&self.config.log_level).unwrap());
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
}
