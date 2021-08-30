pub mod config;

use crate::config::{parse, MercuryConfig};

use core_service::Service;

use ansi_term::Colour::Green;
use clap::{crate_version, App, Arg, ArgMatches, SubCommand};
use log::{info, LevelFilter};
use log4rs::append::{console::ConsoleAppender, file::FileAppender};
use log4rs::config::{Appender, Root};
use log4rs::{encode::pattern::PatternEncoder, Config};

use std::str::FromStr;
use std::time::Duration;

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
                SubCommand::with_name("reset")
                    .about("reset with a height of db checkpoint")
                    .arg(
                        Arg::with_name("height")
                            .short("h")
                            .long("height")
                            .help("find the latest snapshot below this height")
                            .required(true)
                            .takes_value(true),
                    ),
            )
            .get_matches();

        let mut config: MercuryConfig =
            parse(matches.value_of("config_path").expect("missing config")).unwrap();

        config.check();

        Cli { matches, config }
    }

    pub async fn start(&self) {
        match self.matches.subcommand() {
            ("run", None) => self.run().await,
            _ => self.run().await,
        }
    }

    async fn run(&self) {
        self.print_logo();
        self.log_init(false);

        let service = Service::new(
            self.config.store_path.as_str(),
            self.config.listen_uri.as_str(),
            Duration::from_secs(2),
            self.config.rpc_thread_num,
            self.config.network_type.as_str(),
            self.config.flush_cache_interval,
            self.config.to_json_extensions_config().into(),
            self.config.cellbase_maturity,
            self.config.ckb_uri.clone(),
            self.config.cheque_since,
        );

        let rpc_server = service.init();

        service.start().await;

        rpc_server.close();

        info!("Closing!");
    }

    fn log_init(&self, coerce_console: bool) {
        let mut root_builder = Root::builder();
        let log_path = if coerce_console {
            CONSOLE
        } else {
            self.config.log_path.as_str()
        };

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

    fn print_logo(&self) {
        println!(
            "{}",
            format!(
                r#"
  _   _   ______   _____   __      __ {}   _____
 | \ | | |  ____| |  __ \  \ \    / / {}  / ____|
 |  \| | | |__    | |__) |  \ \  / /  {} | (___
 | . ` | |  __|   |  _  /    \ \/ /   {}  \___ \
 | |\  | | |____  | | \ \     \  /    {}  ____) |
 |_| \_| |______| |_|  \_\     \/     {} |_____/
"#,
                Green.bold().paint(r#"  ____  "#),
                Green.bold().paint(r#" / __ \ "#),
                Green.bold().paint(r#"| |  | |"#),
                Green.bold().paint(r#"| |  | |"#),
                Green.bold().paint(r#"| |__| |"#),
                Green.bold().paint(r#" \____/ "#),
            )
        );
    }
}
