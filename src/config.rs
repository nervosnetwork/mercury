use crate::extensions::ExtensionType;
use crate::types::{JsonDeployedScriptConfig, JsonExtensionsConfig};

use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs::File, io::Read};

pub fn parse<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    parse_reader(&mut File::open(name)?)
}

#[derive(Deserialize, Debug)]
pub struct MercuryConfig {
    #[serde(default = "default_ckb_uri")]
    pub ckb_uri: String,

    #[serde(default = "default_listen_uri")]
    pub listen_uri: String,

    #[serde(default = "default_store_path")]
    pub store_path: PathBuf,

    #[serde(default = "default_network_type")]
    pub network_type: String,

    pub extensions_config: Vec<JsonExtConfig>,
}

#[derive(Deserialize, Debug)]
pub struct JsonExtConfig {
    extension_name: String,
    script: String,
    cell_dep: String,
}

impl MercuryConfig {
    pub fn to_json_extensions_config(&self) -> JsonExtensionsConfig {
        let enabled_extensions = self
            .extensions_config
            .iter()
            .map(|c| {
                (
                    ExtensionType::from(c.extension_name.as_str()),
                    JsonDeployedScriptConfig {
                        script: serde_json::from_str(&c.script).unwrap_or_else(|err| {
                            panic!(
                                "Decode {:?} config script error {:?}",
                                c.extension_name, err
                            )
                        }),
                        cell_dep: serde_json::from_str(&c.cell_dep).unwrap_or_else(|e| {
                            panic!(
                                "Decode {:?} config cell dep error {:?}",
                                c.extension_name, e
                            )
                        }),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        JsonExtensionsConfig { enabled_extensions }
    }
}

fn default_ckb_uri() -> String {
    String::from("http://127.0.0.1:8114")
}

fn default_listen_uri() -> String {
    String::from("http://127.0.0.1:8116")
}

fn default_store_path() -> PathBuf {
    PathBuf::from("../free-space/db")
}

fn default_network_type() -> String {
    String::from("ckb")
}

fn parse_reader<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf)?;
    Ok(toml::from_slice(&buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_PATH: &str = "./devtools/config/config.toml";

    #[test]
    fn test_parse() {
        let config: MercuryConfig = parse(PathBuf::from(CONFIG_PATH)).unwrap();
        let json_configs = config.to_json_extensions_config();

        assert_eq!(json_configs.enabled_extensions.len(), 1);

        let sudt_config = json_configs
            .enabled_extensions
            .get(&ExtensionType::SUDTBalacne)
            .cloned()
            .unwrap();

        println!("{:?}", sudt_config.script.code_hash.as_bytes());
    }
}
