use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{Priority, ProcessEvent, ResourceType};

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
pub struct AgentStatus {
    pub online: bool,
    pub cpu_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventMessage {
    Interrupt {
        message: String,
        target_session: Option<String>,
    },

    ProcessNotification {
        process_id: String,
        event: ProcessEvent,
        output_preview: Option<String>,
    },

    Message {
        content: String,
        from: String,
    },

    AssignTask {
        task_id: String,
        title: String,
        description: String,
        priority: Priority,
        deadline: Option<DateTime<Utc>>,
        context: Option<serde_json::Value>,
    },

    Remind {
        id: String,
        message: String,
    },

    Query {
        query_id: String,
        question: String,
    },

    ConfigUpdate {
        llm_base_url: Option<String>,
        llm_api_key: Option<String>,
        llm_model: Option<String>,
    },

    ResourceWarning {
        resource: ResourceType,
        message: String,
    },

    ToolResult {
        tool_call_id: String,
        result: serde_json::Value,
    },

    Error {
        message: String,
    },

    Log {
        level: String,
        message: String,
    },

    StatusUpdate {
        status: String,
    },

    FileRead {
        path: String,
    },

    FileWrite {
        path: String,
        content: String,
    },

    FileList {
        path: String,
    },

    FileDelete {
        path: String,
    },

    ShellExec {
        command: String,
    },

    HttpRequest {
        url: String,
        method: String,
        headers: Option<serde_json::Value>,
        body: Option<serde_json::Value>,
    },

    QueryResponse {
        query_id: String,
        response: String,
    },
}
