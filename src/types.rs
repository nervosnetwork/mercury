use crate::extensions::ExtensionType;

use ckb_jsonrpc_types::{CellDep, Script};
use ckb_types::packed;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub struct JsonDeployedScriptConfig {
    pub script: Script,
    pub cell_dep: CellDep,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct JsonExtensionsConfig {
    pub enabled_extensions: HashMap<ExtensionType, JsonDeployedScriptConfig>,
}

#[derive(Default, Clone, Debug)]
pub struct DeployedScriptConfig {
    pub script: packed::Script,
    pub cell_dep: packed::CellDep,
}

#[derive(Default, Clone, Debug)]
pub struct ExtensionsConfig {
    pub enabled_extensions: HashMap<ExtensionType, DeployedScriptConfig>,
}

impl From<JsonDeployedScriptConfig> for DeployedScriptConfig {
    fn from(json: JsonDeployedScriptConfig) -> DeployedScriptConfig {
        DeployedScriptConfig {
            script: json.script.into(),
            cell_dep: json.cell_dep.into(),
        }
    }
}

impl From<DeployedScriptConfig> for JsonDeployedScriptConfig {
    fn from(config: DeployedScriptConfig) -> JsonDeployedScriptConfig {
        JsonDeployedScriptConfig {
            script: config.script.into(),
            cell_dep: config.cell_dep.into(),
        }
    }
}

impl From<JsonExtensionsConfig> for ExtensionsConfig {
    fn from(json: JsonExtensionsConfig) -> ExtensionsConfig {
        ExtensionsConfig {
            enabled_extensions: json
                .enabled_extensions
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}

impl From<ExtensionsConfig> for JsonExtensionsConfig {
    fn from(config: ExtensionsConfig) -> JsonExtensionsConfig {
        JsonExtensionsConfig {
            enabled_extensions: config
                .enabled_extensions
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
