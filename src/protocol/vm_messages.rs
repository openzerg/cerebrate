use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::agent_messages::AgentStatus;
use super::types::FileTreeData;
use super::types::GitRepo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConnect {
    pub agent_name: String,
    pub internal_token: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHeartbeat {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatusReport {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmFileTree {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: FileTreeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRepoList {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: Vec<GitRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmTaskResult {
    pub agent_name: String,
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmEventAck {
    pub event_id: String,
    pub accepted: bool,
    pub message: Option<String>,
}
