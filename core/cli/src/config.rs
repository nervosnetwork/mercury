use common::anyhow::Result;
use core_extensions::{ExtensionType, JsonDeployedScriptConfig, JsonExtensionsConfig};

use serde::{de::DeserializeOwned, Deserialize};

use std::collections::HashMap;
use std::path::Path;
use std::{fs::File, io::Read};

pub fn parse<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    parse_reader(&mut File::open(name)?)
}

#[derive(Deserialize, Default, Debug)]
pub struct MercuryConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,

    #[serde(default = "default_ckb_uri")]
    pub ckb_uri: String,

    #[serde(default = "default_listen_uri")]
    pub listen_uri: String,

    #[serde(default = "default_store_path")]
    pub store_path: String,

    #[serde(default = "default_rpc_thread_num")]
    pub rpc_thread_num: usize,

    #[serde(default = "default_network_type")]
    pub network_type: String,

    #[serde(default = "default_log_path")]
    pub log_path: String,

    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u64,

    #[serde(default = "default_snapshot_path")]
    pub snapshot_path: String,

    #[serde(default = "default_cellbase_maturity")]
    pub cellbase_maturity: u64,

    #[serde(default = "default_cheque_since")]
    pub cheque_since: u64,

    pub extensions_config: Vec<JsonExtConfig>,
}

#[derive(Deserialize, Debug)]
pub struct JsonExtConfig {
    extension_name: String,
    scripts: Vec<DeployedScript>,
}

#[derive(Deserialize, Debug)]
pub struct DeployedScript {
    name: String,
    script: String,
    cell_dep: String,
}

impl MercuryConfig {
    pub fn to_json_extensions_config(&self) -> JsonExtensionsConfig {
        let enabled_extensions = self
            .extensions_config
            .iter()
            .map(|c| {
                let ty = ExtensionType::from(c.extension_name.as_str());
                let config = c
                    .scripts
                    .iter()
                    .map(|s| {
                        (
                            s.name.clone(),
                            JsonDeployedScriptConfig {
                                name: s.name.clone(),
                                script: serde_json::from_str(&s.script).unwrap_or_else(|err| {
                                    panic!("Decode {:?} config script error {:?}", s.name, err)
                                }),
                                cell_dep: serde_json::from_str(&s.cell_dep).unwrap_or_else(|e| {
                                    panic!("Decode {:?} config cell dep error {:?}", s.name, e)
                                }),
                            },
                        )
                    })
                    .collect::<HashMap<_, _>>();
                (ty, config)
            })
            .collect::<HashMap<_, _>>();

        JsonExtensionsConfig { enabled_extensions }
    }

    pub fn check(&mut self) {
        self.build_uri();
        self.check_path();
        self.check_rpc_thread_num()
    }

    fn build_uri(&mut self) {
        if !self.ckb_uri.starts_with("http") {
            let uri = self.ckb_uri.clone();
            self.ckb_uri = format!("http://{}", uri);
        }
    }

    fn check_path(&self) {
        if self.store_path.contains(&self.snapshot_path)
            || self.snapshot_path.contains(&self.store_path)
        {
            panic!("The store and snapshot paths cannot have a containment relationship.");
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

fn default_store_path() -> String {
    String::from("./free-space/db")
}

fn default_rpc_thread_num() -> usize {
    2usize
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

    static TESTNET_CONFIG_PATH: &str = "./devtools/config/testnet_config.toml";
    static MAINNET_CONFIG_PATH: &str = "./devtools/config/mainnet_config.toml";

    #[test]
    fn test_testnet_config_parse() {
        let config: MercuryConfig = parse(TESTNET_CONFIG_PATH).unwrap();
        let json_configs = config.to_json_extensions_config();

        let _sudt_config = json_configs
            .enabled_extensions
            .get(&ExtensionType::UDTBalance)
            .cloned()
            .unwrap();

        println!("{:?}", config.to_json_extensions_config())
    }

    #[test]
    fn test_mainnet_config_parse() {
        let config: MercuryConfig = parse(MAINNET_CONFIG_PATH).unwrap();
        let json_configs = config.to_json_extensions_config();

        let _sudt_config = json_configs
            .enabled_extensions
            .get(&ExtensionType::UDTBalance)
            .cloned()
            .unwrap();

        println!("{:?}", config.to_json_extensions_config())
    }

    #[test]
    #[should_panic]
    fn test_check_path() {
        let mut config = MercuryConfig {
            store_path: String::from("aaa/bbb/store"),
            snapshot_path: String::from("aaa/bbb/snapshot"),
            ..Default::default()
        };

        config.check_path();
        config.snapshot_path = String::from("~/root/aaa/bbb/store");
        config.check_path();
    }
}
