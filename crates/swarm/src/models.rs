use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub enabled: bool,
    pub container_ip: String,
    pub host_ip: String,
    pub forgejo_username: Option<String>,
    pub internal_token: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgejoUser {
    pub username: String,
    pub password: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub version: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub agents: HashMap<String, Agent>,
    #[serde(default)]
    pub forgejo_users: HashMap<String, ForgejoUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_port() -> u16 {
    17531
}
fn default_container_subnet_base() -> String {
    "10.200".to_string()
}
fn default_forgejo_url() -> String {
    "http://localhost:3000".to_string()
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            port: default_port(),
            container_subnet_base: default_container_subnet_base(),
            forgejo_url: default_forgejo_url(),
            forgejo_token: String::new(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            defaults: Defaults::default(),
            agents: HashMap::new(),
            forgejo_users: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Openai,
    Azure,
    Anthropic,
    Deepseek,
    Moonshot,
    Zhipu,
    Custom,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::Openai),
            "azure" => Some(Self::Azure),
            "anthropic" => Some(Self::Anthropic),
            "deepseek" => Some(Self::Deepseek),
            "moonshot" => Some(Self::Moonshot),
            "zhipu" => Some(Self::Zhipu),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Azure => "azure",
            Self::Anthropic => "anthropic",
            Self::Deepseek => "deepseek",
            Self::Moonshot => "moonshot",
            Self::Zhipu => "zhipu",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub provider_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub provider_id: String,
}
