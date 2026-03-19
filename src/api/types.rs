use crate::models::{CreateApiKeyRequest, CreateProviderRequest, ProviderType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub forgejo_username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateForgejoUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    pub agent: String,
    pub forgejo_user: String,
}

#[derive(Debug, Deserialize)]
pub struct UnbindRequest {
    pub agent: String,
}

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub enabled: bool,
    pub container_ip: String,
    pub host_ip: String,
    pub forgejo_username: Option<String>,
    pub online: bool,
}

#[derive(Debug, Serialize)]
pub struct ForgejoUserInfo {
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    pub exported_at: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_agents: usize,
    pub online_agents: usize,
    pub enabled_agents: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepoRequest {
    pub owner: String,
    pub name: String,
    pub private: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TransferRepoRequest {
    pub new_owner: String,
}

#[derive(Debug, Deserialize)]
pub struct AddCollaboratorRequest {
    pub username: String,
    pub permission: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let resp: ApiResponse<String> = ApiResponse::success("test data".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("test data".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<String> = ApiResponse::error("Something went wrong");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_api_response_serialization() {
        let resp: ApiResponse<i32> = ApiResponse::success(42);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_create_agent_request_deserialization() {
        let json = r#"{"name":"agent1","forgejo_username":"user1"}"#;
        let req: CreateAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "agent1");
        assert_eq!(req.forgejo_username, Some("user1".to_string()));
    }

    #[test]
    fn test_create_agent_request_minimal() {
        let json = r#"{"name":"agent2"}"#;
        let req: CreateAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "agent2");
        assert!(req.forgejo_username.is_none());
    }

    #[test]
    fn test_create_forgejo_user_request() {
        let json = r#"{"username":"newuser","password":"secret123"}"#;
        let req: CreateForgejoUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "newuser");
        assert_eq!(req.password, "secret123");
    }

    #[test]
    fn test_bind_request() {
        let json = r#"{"agent":"agent1","forgejo_user":"user1"}"#;
        let req: BindRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent, "agent1");
        assert_eq!(req.forgejo_user, "user1");
    }

    #[test]
    fn test_unbind_request() {
        let json = r#"{"agent":"agent1"}"#;
        let req: UnbindRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent, "agent1");
    }

    #[test]
    fn test_agent_info_serialization() {
        let info = AgentInfo {
            name: "agent1".to_string(),
            enabled: true,
            container_ip: "10.0.0.2".to_string(),
            host_ip: "10.0.0.1".to_string(),
            forgejo_username: Some("user1".to_string()),
            online: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("agent1"));
        assert!(json.contains("10.0.0.2"));
    }

    #[test]
    fn test_forgejo_user_info() {
        let info = ForgejoUserInfo {
            username: "user1".to_string(),
            email: "user@example.com".to_string(),
            created_at: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("user1"));
    }

    #[test]
    fn test_config_info() {
        let info = ConfigInfo {
            exported_at: "2024-01-01T00:00:00Z".to_string(),
            path: "/etc/zerg/config.yaml".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("/etc/zerg/config.yaml"));
    }

    #[test]
    fn test_provider_info() {
        let info = ProviderInfo {
            id: "provider-1".to_string(),
            name: "OpenAI".to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.openai.com".to_string(),
            enabled: true,
            created_at: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("OpenAI"));
    }

    #[test]
    fn test_stats_summary() {
        let stats = StatsSummary {
            total_agents: 10,
            online_agents: 5,
            enabled_agents: 8,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_create_repo_request() {
        let json = r#"{"owner":"myorg","name":"new-repo","private":true}"#;
        let req: CreateRepoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.owner, "myorg");
        assert_eq!(req.name, "new-repo");
        assert_eq!(req.private, Some(true));
    }

    #[test]
    fn test_transfer_repo_request() {
        let json = r#"{"new_owner":"neworg"}"#;
        let req: TransferRepoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.new_owner, "neworg");
    }

    #[test]
    fn test_add_collaborator_request() {
        let json = r#"{"username":"collab1","permission":"read"}"#;
        let req: AddCollaboratorRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "collab1");
        assert_eq!(req.permission, Some("read".to_string()));
    }

    #[test]
    fn test_create_org_request() {
        let json = r#"{"name":"neworg"}"#;
        let req: CreateOrgRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "neworg");
    }
}
