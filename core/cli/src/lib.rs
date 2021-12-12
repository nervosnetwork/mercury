pub mod config;

use crate::config::{parse, MercuryConfig};

use common_logger::init_jaeger;
use core_service::Service;

use ansi_term::Colour::Green;
use clap::{crate_version, App, Arg, ArgMatches, SubCommand};
use log::{info, LevelFilter};

use std::path::PathBuf;
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
            .arg(
                Arg::with_name("db_user")
                    .long("db_user")
                    .help("Mercury database user name")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("db_pwd")
                    .long("db_pwd")
                    .help("Mercury database user password")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("db_host")
                    .long("db_host")
                    .help("Mercury database host")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("db_port")
                    .long("db_port")
                    .help("Mercury database port")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("listen_uri")
                    .long("listen_uri")
                    .help("Mercury listen uri")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("ckb_uri")
                    .long("ckb_uri")
                    .help("Mercury ckb uri")
                    .takes_value(true),
            )
            .subcommand(SubCommand::with_name("run").about("run the mercury process"))
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

    fn parse_cmd_args<T: FromStr>(&self, cmd_arg_name: &str, config_value: T) -> T {
        if let Some(arg) = self.matches.value_of(cmd_arg_name) {
            if let Ok(res) = arg.parse() {
                res
            } else {
                panic!("Invalid command argument: {}", cmd_arg_name)
            }
        } else {
            config_value
        }
    }

    async fn run(&self) {
        self.print_logo();
        self.log_init();

        if self.config.log_config.use_apm {
            init_jaeger(self.config.log_config.jaeger_uri.clone().unwrap());
        }

        let mut service = Service::new(
            self.config.db_config.center_id,
            self.config.db_config.machine_id,
            self.config.db_config.max_connections,
            self.config.db_config.min_connections,
            self.config.db_config.connect_timeout,
            self.config.db_config.max_lifetime,
            self.config.db_config.idle_timeout,
            Duration::from_secs(2),
            self.config.rpc_thread_num,
            &self.config.network_config.network_type,
            self.config.use_tx_pool_cache,
            self.config.to_script_map(),
            self.config.cellbase_maturity,
            self.parse_cmd_args("ckb_uri", self.config.network_config.ckb_uri.clone()),
            self.config.cheque_since,
            LevelFilter::from_str(&self.config.db_config.db_log_level).unwrap(),
        );

        let stop_handle = service
            .init(
                self.parse_cmd_args("listen_uri", self.config.network_config.listen_uri.clone()),
                self.config.db_config.db_type.clone(),
                self.config.db_config.db_name.clone(),
                self.parse_cmd_args("db_host", self.config.db_config.db_host.clone()),
                self.parse_cmd_args("db_port", self.config.db_config.db_port),
                self.parse_cmd_args("db_user", self.config.db_config.db_user.clone()),
                self.parse_cmd_args("db_pwd", self.config.db_config.password.clone()),
            )
            .await;

        if self.config.allow_parallel_sync {
            service
                .do_sync(
                    self.config.sync_config.sync_block_batch_size,
                    self.config.sync_config.max_task_number,
                )
                .await
                .unwrap();
        }

        if self.config.sync_mode {
            service
                .start(self.config.flush_tx_pool_cache_interval)
                .await;
        } else {
            service.start_rpc_mode().await.unwrap();
        }

        stop_handle.stop().unwrap().await.unwrap();
        info!("Closing!");
    }

    fn log_init(&self) {
        let is_output_console = self.config.log_config.log_path.as_str() == CONSOLE;
        common_logger::init(
            self.config.log_config.log_level.clone(),
            is_output_console,
            true,
            !is_output_console,
            self.config.log_config.use_metrics,
            PathBuf::from(&self.config.log_config.log_path),
            self.config.log_config.file_size_limit,
            self.config.log_config.module_level.clone(),
        );
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
