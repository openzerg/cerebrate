use serde::{Deserialize, Serialize};

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
