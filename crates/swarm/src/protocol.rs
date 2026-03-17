use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    VmConnect(VmConnect),
    VmHeartbeat(VmHeartbeat),
    VmStatusReport(VmStatusReport),
    VmFileTree(VmFileTree),
    VmRepoList(VmRepoList),
    VmTaskResult(VmTaskResult),

    HostExecuteTask(HostExecuteTask),
    HostConfigUpdate(HostConfigUpdate),
    HostRequestFiles(HostRequestFiles),
    HostRequestRepos(HostRequestRepos),

    AgentEvent(AgentEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub event: AgentEventType,
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Connected,
    Disconnected,
    StatusUpdate,
    Created,
    Deleted,
    Enabled,
    Disabled,
    ConfigApplying,
    ConfigApplied,
    ConfigError,
}

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
pub struct AgentStatus {
    pub online: bool,
    pub cpu_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmFileTree {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: FileTreeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeData {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRepoList {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: Vec<GitRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepo {
    pub path: String,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub status: String,
    pub ahead: u32,
    pub behind: u32,
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

impl Message {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
