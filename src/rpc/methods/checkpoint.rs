use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("checkpoint.create", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { 
                agent: String, 
                #[serde(default)]
                description: Option<String> 
            }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
                &state.data_dir,
                "/dev/sda2",
                std::path::Path::new("/home"),
            );
            
            let checkpoint_id = checkpoint_mgr.create_checkpoint(&p.agent, p.description.as_deref().unwrap_or(""))
                .await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let (_, meta) = state.state_manager.load_checkpoint(&checkpoint_id).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::to_value(meta).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("checkpoint.list", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { #[serde(default)] agent: Option<String> }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
                &state.data_dir,
                "/dev/sda2",
                std::path::Path::new("/home"),
            );
            
            let checkpoints = checkpoint_mgr.list_checkpoints(p.agent.as_deref()).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::to_value(checkpoints).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("checkpoint.rollback", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, checkpoint_id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
                &state.data_dir,
                "/dev/sda2",
                std::path::Path::new("/home"),
            );
            
            checkpoint_mgr.rollback(&p.agent, &p.checkpoint_id).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"rolled_back": true, "message": "Please run 'zerg-swarm apply' to rebuild"}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("checkpoint.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
                &state.data_dir,
                "/dev/sda2",
                std::path::Path::new("/home"),
            );
            
            checkpoint_mgr.delete_checkpoint(&p.id).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("checkpoint.clone", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String, new_name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
                &state.data_dir,
                "/dev/sda2",
                std::path::Path::new("/home"),
            );
            
            checkpoint_mgr.clone(&p.id, &p.new_name).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let agent = sw.agents.get(&p.new_name)
                .ok_or_else(|| RpcError::internal_error("Agent not found after clone"))?;
            
            Ok(serde_json::to_value(agent).unwrap())
        }
    }).await;
}