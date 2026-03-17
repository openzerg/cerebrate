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
