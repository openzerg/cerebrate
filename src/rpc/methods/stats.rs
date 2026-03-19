use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("stats.summary", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let connections = state.vm_connections.read().await;
            let online_agents = connections.values().filter(|c| c.connected).count();
            
            Ok(serde_json::json!({
                "total_agents": sw.agents.len(),
                "online_agents": online_agents,
                "enabled_agents": sw.agents.values().filter(|a| a.enabled).count(),
            }))
        }
    }).await;
}