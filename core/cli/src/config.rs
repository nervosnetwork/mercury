use common::anyhow::Result;

use ckb_jsonrpc_types::{CellDep, Script};
use serde::{de::DeserializeOwned, Deserialize};

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;
use std::{fs::File, io::Read};

pub type JsonString = String;

pub fn parse<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    parse_reader(&mut File::open(name)?)
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct NetworkConfig {
    #[serde(default = "default_network_type")]
    pub network_type: String,

    #[serde(default = "default_ckb_uri")]
    pub ckb_uri: String,

    #[serde(default = "default_listen_uri")]
    pub listen_uri: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct DBConfig {
    pub db_type: String,
    pub db_path: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub password: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct LogConfig {
    #[serde(default = "default_log_path")]
    pub log_path: String,
    pub log_level: String,
    pub use_split_file: bool,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ScriptConfig {
    pub script_name: String,
    pub script: Script,
    pub cell_dep: CellDep,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ExtensionConfig {
    extension_name: String,
    config: JsonString,
}

#[derive(Deserialize, Default, Debug)]
pub struct MercuryConfig {
    pub center_id: u16,
    pub machine_id: u16,
    pub db_config: DBConfig,
    pub log_config: LogConfig,
    pub network_config: NetworkConfig,
    pub builtin_scripts: Vec<ScriptConfig>,

    #[serde(default = "default_rpc_thread_num")]
    pub rpc_thread_num: usize,

    #[serde(default = "default_flush_tx_pool_cache_interval")]
    pub flush_tx_pool_cache_interval: usize,

    #[serde(default = "default_cheque_since")]
    pub cheque_since: u64,

    #[serde(default = "default_cellbase_maturity")]
    pub cellbase_maturity: u64,
    pub extensions_config: Vec<ExtensionConfig>,
}

impl MercuryConfig {
    pub fn check(&mut self) {
        self.build_uri();
        self.check_rpc_thread_num()
    }

    fn build_uri(&mut self) {
        if !self.ckb_uri.starts_with("http") {
            let uri = self.ckb_uri.clone();
            self.ckb_uri = format!("http://{}", uri);
        }
    }

    fn check_rpc_thread_num(&self) {
        if self.rpc_thread_num < 2 {
            panic!("The rpc thread number must be at least 2");
        }
    }
}

fn default_log_level() -> String {
    String::from("INFO")
}

fn default_ckb_uri() -> String {
    String::from("http://127.0.0.1:8114")
}

fn default_listen_uri() -> String {
    String::from("127.0.0.1:8116")
}

fn default_rpc_thread_num() -> usize {
    2usize
}

fn default_flush_tx_pool_cache_interval() -> usize {
    300
}

fn default_network_type() -> String {
    String::from("ckb")
}

fn default_log_path() -> String {
    String::from("console")
}

fn default_snapshot_interval() -> u64 {
    5000
}

fn default_snapshot_path() -> String {
    String::from("./free-space/snapshot")
}

fn default_cellbase_maturity() -> u64 {
    4u64
}

fn default_cheque_since() -> u64 {
    6u64
}

fn parse_reader<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf)?;
    Ok(toml::from_slice(&buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TESTNET_CONFIG_PATH: &str = "../../devtools/config/testnet_config.toml";
    static MAINNET_CONFIG_PATH: &str = "../../devtools/config/mainnet_config.toml";

    #[test]
    fn test_testnet_config_parse() {
        let config: MercuryConfig = parse(TESTNET_CONFIG_PATH).unwrap();

        println!("{:?}", config)
    }

    #[test]
    fn test_mainnet_config_parse() {
        let config: MercuryConfig = parse(MAINNET_CONFIG_PATH).unwrap();

        println!("{:?}", config)
    }
}
