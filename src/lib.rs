pub mod api;
pub mod agent_manager;
pub mod state;
pub mod checkpoint;
pub mod incus;
pub mod config;
pub mod forgejo;
pub mod proxy;
pub mod protocol;
pub mod models;
pub mod error;
pub mod tool_manager;
pub mod sync;
pub mod llm_proxy;
pub mod grpc;
pub mod jwt;
mod app_state_impl;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use crate::protocol::AgentEvent;
use crate::grpc::AgentGrpcClient;

pub use error::{Error, Result};
pub use models::*;

pub struct AppState {
    pub state_manager: state::StateManager,
    pub agent_manager: agent_manager::AgentManager,
    pub tool_manager: tool_manager::ToolManager,
    pub vm_connections: RwLock<HashMap<String, VmConnection>>,
    pub event_tx: broadcast::Sender<AgentEvent>,
    pub data_dir: std::path::PathBuf,
    pub apply_tx: mpsc::UnboundedSender<()>,
    pub grpc_client: Arc<AgentGrpcClient>,
}

pub struct VmConnection {
    pub agent_name: String,
    pub connected: bool,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub agent_ip: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState {
            state_manager: state::StateManager::new(std::path::Path::new("/tmp")),
            agent_manager: agent_manager::AgentManager::new(std::path::Path::new("/tmp")),
            tool_manager: tool_manager::ToolManager::new(
                std::path::PathBuf::from("/tmp"),
                "http://localhost:3000".to_string(),
                "".to_string(),
            ),
            vm_connections: RwLock::new(HashMap::new()),
            event_tx: broadcast::channel(100).0,
            data_dir: std::path::PathBuf::from("/tmp"),
            apply_tx: mpsc::unbounded_channel().0,
            grpc_client: Arc::new(AgentGrpcClient::new()),
        };
        assert!(state.data_dir.to_str().unwrap().contains("tmp"));
    }

    #[test]
    fn test_vm_connection_creation() {
        let conn = VmConnection {
            agent_name: "agent-1".to_string(),
            connected: true,
            last_heartbeat: chrono::Utc::now(),
            agent_ip: "127.0.0.1".to_string(),
        };
        assert_eq!(conn.agent_name, "agent-1");
        assert!(conn.connected);
    }
}