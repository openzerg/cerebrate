use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use cerebrate::{AppState, Result, PylonClient};
use cerebrate::state;
use cerebrate::agent_manager;
use cerebrate::tool_manager;
use cerebrate::protocol;
use cerebrate::grpc::AgentGrpcClient;

pub async fn init_state(data_dir: std::path::PathBuf) -> Result<Arc<AppState>> {
    tokio::fs::create_dir_all(&data_dir).await?;

    let system_dir = data_dir.join("system");
    tokio::fs::create_dir_all(&system_dir).await?;

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
    
    let grpc_client = Arc::new(AgentGrpcClient::new());
    let pylon_client = Arc::new(PylonClient::new());

    let state = Arc::new(AppState {
        state_manager,
        agent_manager,
        tool_manager,
        vm_connections: RwLock::new(HashMap::new()),
        event_tx,
        data_dir: data_dir.clone(),
        apply_tx,
        grpc_client,
        pylon_client,
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
    while let Some(_) = apply_rx.recv().await {
        tracing::info!("Applying Incus container configuration...");
        
        let _ = state.event_tx.send(protocol::AgentEvent {
            event: protocol::AgentEventType::ConfigApplying,
            agent_name: "system".to_string(),
            timestamp: chrono::Utc::now(),
            data: None,
        });
        
        match state.state_manager.load().await {
            Ok(sw) => {
                if let Err(e) = state.agent_manager.apply(&sw).await {
                    tracing::error!("Failed to apply configuration: {}", e);
                    let _ = state.event_tx.send(protocol::AgentEvent {
                        event: protocol::AgentEventType::ConfigError,
                        agent_name: "system".to_string(),
                        timestamp: chrono::Utc::now(),
                        data: Some(serde_json::json!({ "error": e.to_string() })),
                    });
                } else {
                    tracing::info!("Incus containers configured successfully");
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