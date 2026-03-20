use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub description: String,
    pub status: String,
    pub status_code: i64,
    #[serde(rename = "type")]
    pub instance_type: String,
    pub architecture: String,
    pub profiles: Vec<String>,
    pub stateful: bool,
    pub created_at: String,
    pub location: String,
    pub project: String,
    pub config: Option<HashMap<String, String>>,
    pub devices: Option<HashMap<String, HashMap<String, String>>>,
    pub ephemeral: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub status: String,
    pub status_code: i64,
    pub disk: HashMap<String, serde_json::Value>,
    pub memory: serde_json::Value,
    pub network: HashMap<String, InstanceNetwork>,
    pub pid: i64,
    pub processes: i64,
    pub cpu: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSnapshot {
    pub name: String,
    pub created_at: String,
    pub stateful: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,
    pub source: InstanceSource,
    pub config: Option<HashMap<String, String>>,
    pub devices: Option<HashMap<String, HashMap<String, String>>>,
    pub profiles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub alias: Option<String>,
    pub protocol: Option<String>,
    pub server: Option<String>,
}

impl InstanceSource {
    pub fn image(alias: &str) -> Self {
        Self {
            source_type: "image".to_string(),
            alias: Some(alias.to_string()),
            protocol: None,
            server: None,
        }
    }

    pub fn image_from_remote(alias: &str, server: &str) -> Self {
        Self {
            source_type: "image".to_string(),
            alias: Some(alias.to_string()),
            protocol: Some("simplestreams".to_string()),
            server: Some(server.to_string()),
        }
    }
}

impl InstanceConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            source: InstanceSource::image("nixos/25.11"),
            config: None,
            devices: None,
            profiles: None,
        }
    }

    pub fn with_image(mut self, alias: &str) -> Self {
        self.source = InstanceSource::image(alias);
        self
    }

    pub fn with_profiles(mut self, profiles: Vec<String>) -> Self {
        self.profiles = Some(profiles);
        self
    }

    pub fn with_config(mut self, config: HashMap<String, String>) -> Self {
        self.config = Some(config);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub class: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub status: String,
    pub status_code: i64,
    pub resources: Option<HashMap<String, Vec<String>>>,
    pub metadata: Option<serde_json::Value>,
    pub may_cancel: Option<bool>,
    pub err: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAddress {
    pub family: String,
    pub address: String,
    pub netmask: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceNetwork {
    pub addresses: Vec<NetworkAddress>,
    pub counters: Option<serde_json::Value>,
    pub hwaddr: String,
    pub host_name: String,
    pub mtu: i64,
    pub state: String,
    #[serde(rename = "type")]
    pub network_type: String,
}
