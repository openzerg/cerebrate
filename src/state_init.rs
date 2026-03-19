use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use swarm::{AppState, Result};
use swarm::state;
use swarm::agent_manager;
use swarm::tool_manager;
use swarm::protocol;

pub async fn init_state(data_dir: std::path::PathBuf) -> Result<Arc<AppState>> {
    tokio::fs::create_dir_all(&data_dir).await?;

    let system_dir = data_dir.join("system");
    let generated_dir = system_dir.join("generated");
    tokio::fs::create_dir_all(&generated_dir).await?;

    let state_manager = state::StateManager::new(&data_dir);
    let agent_manager = agent_manager::AgentManager::new(&system_dir);
    
    let mut sw = state_manager.load().await?;
    
    if sw.admin_token.is_none() {
        let token = uuid::Uuid::new_v4().to_string();
        sw.admin_token = Some(token);
        state_manager.save(&sw).await?;
    }
    
    let forgejo_url = sw.defaults.forgejo_url.clone();
    let forgejo_token = sw.defaults.forgejo_token.clone();
    
    let tool_manager = tool_manager::ToolManager::new(data_dir.clone(), forgejo_url, forgejo_token);
    let (event_tx, _) = tokio::sync::broadcast::channel::<protocol::AgentEvent>(256);
    let (apply_tx, apply_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let state = Arc::new(AppState {
        state_manager,
        agent_manager,
        tool_manager,
        vm_connections: RwLock::new(HashMap::new()),
        pending_tool_results: RwLock::new(HashMap::new()),
        pending_queries: RwLock::new(HashMap::new()),
        event_tx,
        data_dir: data_dir.clone(),
        apply_tx,
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        handle_apply_tasks(state_clone, apply_rx).await;
    });

    Ok(state)
}

async fn handle_apply_tasks(
    state: Arc<AppState>,
    mut apply_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
) {
    let btrfs_device = std::env::var("ZERG_SWARM_BTRFS_DEVICE")
        .unwrap_or_else(|_| "/dev/sda2".to_string());
    
    while let Some(_) = apply_rx.recv().await {
        tracing::info!("Applying NixOS configuration...");
        
        let _ = state.event_tx.send(protocol::AgentEvent {
            event: protocol::AgentEventType::ConfigApplying,
            agent_name: "system".to_string(),
            timestamp: chrono::Utc::now(),
            data: None,
        });
        
        match state.state_manager.load().await {
            Ok(sw) => {
                if let Err(e) = state.agent_manager.apply(&sw, &btrfs_device).await {
                    tracing::error!("Failed to apply configuration: {}", e);
                    let _ = state.event_tx.send(protocol::AgentEvent {
                        event: protocol::AgentEventType::ConfigError,
                        agent_name: "system".to_string(),
                        timestamp: chrono::Utc::now(),
                        data: Some(serde_json::json!({ "error": e.to_string() })),
                    });
                } else {
                    tracing::info!("NixOS configuration applied successfully");
                    let _ = state.event_tx.send(protocol::AgentEvent {
                        event: protocol::AgentEventType::ConfigApplied,
                        agent_name: "system".to_string(),
                        timestamp: chrono::Utc::now(),
                        data: None,
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to load state: {}", e);
                let _ = state.event_tx.send(protocol::AgentEvent {
                    event: protocol::AgentEventType::ConfigError,
                    agent_name: "system".to_string(),
                    timestamp: chrono::Utc::now(),
                    data: Some(serde_json::json!({ "error": e.to_string() })),
                });
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        while apply_rx.try_recv().is_ok() {}
    }
}