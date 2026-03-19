use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeData {
    pub path: String,
    pub entries: Vec<FileEntry>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessEvent {
    Started,
    Completed { exit_code: i32 },
    Failed { error: String },
    OutputLimitReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Processes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeToolResponse {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSkillResult {
    pub agent_name: String,
    pub skill_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostExecuteSkill {
    pub skill_id: String,
    pub skill_name: String,
    pub entrypoint: String,
    pub skill_files: Vec<(String, String)>,
    pub input: serde_json::Value,
    pub timeout_secs: u64,
}
