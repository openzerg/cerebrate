use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_container_subnet_base")]
    pub container_subnet_base: String,
    #[serde(default = "default_forgejo_url")]
    pub forgejo_url: String,
    #[serde(default)]
    pub forgejo_token: String,
}

pub fn default_port() -> u16 {
    17531
}
pub fn default_container_subnet_base() -> String {
    "192.168.200".to_string()
}
pub fn default_forgejo_url() -> String {
    "http://localhost:3000".to_string()
}
