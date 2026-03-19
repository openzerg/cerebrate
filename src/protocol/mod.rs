mod agent_messages;
mod host_messages;
mod tests;
mod types;
mod vm_messages;

use serde::{Deserialize, Serialize};

pub use agent_messages::{AgentEvent, AgentEventMessage, AgentEventType, AgentStatus};
pub use host_messages::{
    HostConfigUpdate, HostEvent, HostExecuteTask, HostRequestFiles, HostRequestRepos,
};
pub use types::{
    FileEntry, FileTreeData, GitRepo, HostExecuteSkill, InvokeToolResponse, Priority, ProcessEvent,
    ResourceType, VmSkillResult,
};
pub use vm_messages::{
    VmConnect, VmEventAck, VmFileTree, VmHeartbeat, VmRepoList, VmStatusReport, VmTaskResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    VmConnect(VmConnect),
    VmHeartbeat(VmHeartbeat),
    VmStatusReport(VmStatusReport),
    VmFileTree(VmFileTree),
    VmRepoList(VmRepoList),
    VmTaskResult(VmTaskResult),
    VmSkillResult(VmSkillResult),
    VmEventAck(VmEventAck),

    HostExecuteTask(HostExecuteTask),
    HostConfigUpdate(HostConfigUpdate),
    HostRequestFiles(HostRequestFiles),
    HostRequestRepos(HostRequestRepos),
    HostExecuteSkill(HostExecuteSkill),
    HostEvent(HostEvent),

    AgentEvent(AgentEvent),
}

impl Message {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
