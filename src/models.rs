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
    pub models: HashMap<String, Model>,
    #[serde(default)]
    pub forgejo_users: HashMap<String, ForgejoUser>,
    #[serde(default)]
    pub skills: HashMap<String, Skill>,
    #[serde(default)]
    pub tools: HashMap<String, Tool>,
    #[serde(default)]
    pub admin_token: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CallerIdentity {
    Admin,
    Agent(String),
}

impl State {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            defaults: Defaults::default(),
            agents: HashMap::new(),
            providers: HashMap::new(),
            models: HashMap::new(),
            forgejo_users: HashMap::new(),
            skills: HashMap::new(),
            tools: HashMap::new(),
            admin_token: None,
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
    pub model_id: Option<String>,
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
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_name: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub provider_id: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub id: String,
    pub agent_name: String,
    pub description: String,
    pub created_at: String,
    pub btrfs_snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub entrypoint: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    pub author_agent: String,
    #[serde(default)]
    pub allowed_agents: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateToolRequest {
    pub slug: String,
    pub author_agent: String,
    pub forgejo_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeToolRequest {
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeToolResponse {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetEnvRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: String,
    pub entrypoint: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub author_agent: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub slug: String,
    pub author_agent: String,
    pub forgejo_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_new() {
        let state = State::new();
        assert_eq!(state.version, "1.0");
        assert!(state.agents.is_empty());
        assert!(state.providers.is_empty());
        assert!(state.models.is_empty());
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert!(state.version.is_empty()); // Default::default for String
    }

    #[test]
    fn test_agent_serialization() {
        let agent = Agent {
            enabled: true,
            container_ip: "10.200.1.2".to_string(),
            host_ip: "10.200.1.1".to_string(),
            forgejo_username: Some("user".to_string()),
            internal_token: "token123".to_string(),
            model_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("10.200.1.2"));
        assert!(json.contains("token123"));
    }

    #[test]
    fn test_agent_deserialization() {
        let json = r#"{"enabled":true,"container_ip":"10.200.1.2","host_ip":"10.200.1.1","forgejo_username":null,"internal_token":"token","model_id":null,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
        let agent: Agent = serde_json::from_str(json).unwrap();
        assert!(agent.enabled);
        assert_eq!(agent.container_ip, "10.200.1.2");
    }

    #[test]
    fn test_defaults_default() {
        let defaults = Defaults::default();
        assert_eq!(defaults.port, 0); // Default::default for u16
        assert!(defaults.container_subnet_base.is_empty());
        assert!(defaults.forgejo_url.is_empty());
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(ProviderType::from_str("openai"), Some(ProviderType::Openai));
        assert_eq!(ProviderType::from_str("azure"), Some(ProviderType::Azure));
        assert_eq!(
            ProviderType::from_str("anthropic"),
            Some(ProviderType::Anthropic)
        );
        assert_eq!(
            ProviderType::from_str("deepseek"),
            Some(ProviderType::Deepseek)
        );
        assert_eq!(
            ProviderType::from_str("moonshot"),
            Some(ProviderType::Moonshot)
        );
        assert_eq!(ProviderType::from_str("zhipu"), Some(ProviderType::Zhipu));
        assert_eq!(ProviderType::from_str("custom"), Some(ProviderType::Custom));
        assert_eq!(ProviderType::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_type_from_str_case_insensitive() {
        assert_eq!(ProviderType::from_str("OPENAI"), Some(ProviderType::Openai));
        assert_eq!(ProviderType::from_str("Azure"), Some(ProviderType::Azure));
    }

    #[test]
    fn test_provider_type_as_str() {
        assert_eq!(ProviderType::Openai.as_str(), "openai");
        assert_eq!(ProviderType::Azure.as_str(), "azure");
        assert_eq!(ProviderType::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderType::Deepseek.as_str(), "deepseek");
        assert_eq!(ProviderType::Moonshot.as_str(), "moonshot");
        assert_eq!(ProviderType::Zhipu.as_str(), "zhipu");
        assert_eq!(ProviderType::Custom.as_str(), "custom");
    }

    #[test]
    fn test_provider_type_serialization() {
        let pt = ProviderType::Openai;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"openai\"");
    }

    #[test]
    fn test_provider_type_deserialization() {
        let json = "\"azure\"";
        let pt: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(pt, ProviderType::Azure);
    }

    #[test]
    fn test_provider_serialization() {
        let provider = Provider {
            id: "p1".to_string(),
            name: "OpenAI".to_string(),
            provider_type: ProviderType::Openai,
            base_url: "https://api.openai.com".to_string(),
            api_key: "key123".to_string(),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("OpenAI"));
        assert!(json.contains("openai"));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            slug: "my-tool".to_string(),
            name: "My Tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            forgejo_repo: "tools/my-tool".to_string(),
            git_commit: "abc123".to_string(),
            entrypoint: "main.sh".to_string(),
            input_schema: Some(serde_json::json!({"type": "object"})),
            output_schema: None,
            author_agent: "agent-1".to_string(),
            allowed_agents: vec!["agent-1".to_string()],
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("my-tool"));
        assert!(json.contains("A test tool"));
    }

    #[test]
    fn test_skill_serialization() {
        let skill = Skill {
            slug: "my-skill".to_string(),
            name: "My Skill".to_string(),
            version: "2.0.0".to_string(),
            description: "A test skill".to_string(),
            forgejo_repo: "skills/my-skill".to_string(),
            git_commit: "def456".to_string(),
            author_agent: "agent-2".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("my-skill"));
        assert!(json.contains("A test skill"));
    }

    #[test]
    fn test_checkpoint_meta_serialization() {
        let cp = CheckpointMeta {
            id: "cp-1".to_string(),
            agent_name: "agent-1".to_string(),
            description: "Before update".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            btrfs_snapshot: "/snapshots/cp-1".to_string(),
        };
        let json = serde_json::to_string(&cp).unwrap();
        assert!(json.contains("cp-1"));
        assert!(json.contains("Before update"));
    }

    #[test]
    fn test_tool_metadata_serialization() {
        let meta = ToolMetadata {
            name: "Tool".to_string(),
            slug: "tool".to_string(),
            version: "1.0".to_string(),
            description: "Desc".to_string(),
            entrypoint: "run.sh".to_string(),
            input_schema: None,
            output_schema: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("Tool"));
    }

    #[test]
    fn test_skill_metadata_serde() {
        let meta = SkillMetadata {
            name: "Test Skill".to_string(),
            slug: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A test skill".to_string(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let parsed: SkillMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test Skill");
        assert_eq!(parsed.slug, "test-skill");
    }

    #[test]
    fn test_checkpoint_meta_serde() {
        let meta = CheckpointMeta {
            id: "cp_test".to_string(),
            agent_name: "agent-1".to_string(),
            description: "Test checkpoint".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            btrfs_snapshot: "@snapshots/cp_test".to_string(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let parsed: CheckpointMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "cp_test");
    }

    #[test]
    fn test_tool_metadata_serde() {
        let meta = ToolMetadata {
            name: "Test Tool".to_string(),
            slug: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            entrypoint: "python main.py".to_string(),
            input_schema: None,
            output_schema: None,
        };

        let json = serde_json::to_string(&meta).unwrap();
        let parsed: ToolMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test Tool");
    }

    #[test]
    fn test_tool_metadata_with_schemas() {
        let meta = ToolMetadata {
            name: "Tool".to_string(),
            slug: "tool".to_string(),
            version: "1.0".to_string(),
            description: "".to_string(),
            entrypoint: "./run".to_string(),
            input_schema: Some(serde_json::json!({"type": "object"})),
            output_schema: Some(serde_json::json!({"type": "string"})),
        };

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("input_schema"));
        assert!(json.contains("output_schema"));
    }

    #[test]
    fn test_forgejo_user_serde() {
        let user = ForgejoUser {
            username: "testuser".to_string(),
            password: "secret".to_string(),
            email: "test@example.com".to_string(),
            created_at: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&user).unwrap();
        let parsed: ForgejoUser = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.username, "testuser");
    }

    #[test]
    fn test_create_tool_request_serde() {
        let req = CreateToolRequest {
            slug: "new-tool".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/tool".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: CreateToolRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.slug, "new-tool");
    }

    #[test]
    fn test_create_skill_request_serde() {
        let req = CreateSkillRequest {
            slug: "new-skill".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/skill".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: CreateSkillRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.slug, "new-skill");
    }

    #[test]
    fn test_invoke_tool_response_success() {
        let resp = InvokeToolResponse {
            success: true,
            output: Some(serde_json::json!({"result": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("result"));
    }

    #[test]
    fn test_invoke_tool_response_error() {
        let resp = InvokeToolResponse {
            success: false,
            output: None,
            error: Some("Something went wrong".to_string()),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_state_with_all_fields() {
        let json = r#"{
            "version": "2.0",
            "agents": {},
            "providers": {},
            "models": {},
            "tools": {},
            "skills": {},
            "forgejo_users": {},
            "defaults": {
                "port": 8080,
                "container_subnet_base": "10.100",
                "forgejo_url": "http://forgejo:3000",
                "forgejo_token": "token123"
            }
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.version, "2.0");
        assert_eq!(state.defaults.port, 8080);
        assert_eq!(state.defaults.container_subnet_base, "10.100");
    }

    #[test]
    fn test_caller_identity() {
        let admin = CallerIdentity::Admin;
        let agent = CallerIdentity::Agent("agent-1".to_string());

        match admin {
            CallerIdentity::Admin => assert!(true),
            _ => panic!("Wrong variant"),
        }

        match agent {
            CallerIdentity::Agent(name) => assert_eq!(name, "agent-1"),
            _ => panic!("Wrong variant"),
        }
    }
}
