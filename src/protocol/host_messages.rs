use serde::{Deserialize, Serialize};

use super::agent_messages::AgentEventMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostExecuteTask {
    pub task_id: String,
    pub command: String,
    pub cwd: Option<String>,
    pub env: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfigUpdate {
    pub api_key: Option<String>,
    pub git_username: Option<String>,
    pub git_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostRequestFiles {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostRequestRepos;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEvent {
    pub event_id: String,
    pub event: AgentEventMessage,
}
