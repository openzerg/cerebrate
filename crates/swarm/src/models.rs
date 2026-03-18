use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub version: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub agents: HashMap<String, Agent>,
    #[serde(default)]
    pub providers: HashMap<String, Provider>,
    #[serde(default)]
    pub api_keys: HashMap<String, ApiKey>,
    #[serde(default)]
    pub forgejo_users: HashMap<String, ForgejoUser>,
    #[serde(default)]
    pub skills: HashMap<String, Skill>,
}

impl State {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            defaults: Defaults::default(),
            agents: HashMap::new(),
            providers: HashMap::new(),
            api_keys: HashMap::new(),
            forgejo_users: HashMap::new(),
            skills: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub id: String,
    pub agent_name: String,
    pub description: String,
    pub created_at: String,
    pub btrfs_snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    HostScript,
    AgentScript,
}

impl SkillType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hostscript" | "host_script" => Some(Self::HostScript),
            "agentscript" | "agent_script" => Some(Self::AgentScript),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HostScript => "host_script",
            Self::AgentScript => "agent_script",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub enabled: bool,
    pub owner_agent: String,
    #[serde(default)]
    pub allowed_agents: Vec<String>,
    pub entrypoint: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub owner_agent: String,
    pub entrypoint: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeSkillRequest {
    pub input: serde_json::Value,
    pub caller_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeSkillResponse {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}
