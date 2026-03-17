use std::sync::Arc;
use axum::{
    Json, extract::{Path, State},
};
use crate::AppState;
use super::types::ApiResponse;
use crate::models::CheckpointMeta;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateCheckpointRequest {
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub checkpoint_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneCheckpointRequest {
    pub new_name: String,
}

pub async fn create_checkpoint(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(req): Json<CreateCheckpointRequest>,
) -> Json<ApiResponse<CheckpointMeta>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.create_checkpoint(&agent_name, req.description.as_deref().unwrap_or("")).await {
        Ok(checkpoint_id) => {
            let meta = crate::state::StateManager::new(&state.data_dir)
                .load_checkpoint(&checkpoint_id).await;
            match meta {
                Ok((_, meta)) => Json(ApiResponse::success(meta)),
                Err(e) => Json(ApiResponse::error(e.to_string())),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn list_checkpoints(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
) -> Json<ApiResponse<Vec<CheckpointMeta>>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.list_checkpoints(Some(&agent_name)).await {
        Ok(checkpoints) => Json(ApiResponse::success(checkpoints)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn rollback_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> Json<ApiResponse<String>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.rollback(&agent_name, &req.checkpoint_id).await {
        Ok(()) => Json(ApiResponse::success("Rollback successful. Please run 'zerg-swarm apply' to rebuild.".to_string())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn delete_checkpoint(
    State(state): State<Arc<AppState>>,
    Path(checkpoint_id): Path<String>,
) -> Json<ApiResponse<String>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.delete_checkpoint(&checkpoint_id).await {
        Ok(()) => Json(ApiResponse::success(format!("Checkpoint {} deleted", checkpoint_id))),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn clone_checkpoint(
    State(state): State<Arc<AppState>>,
    Path(checkpoint_id): Path<String>,
    Json(req): Json<CloneCheckpointRequest>,
) -> Json<ApiResponse<crate::models::Agent>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.clone(&checkpoint_id, &req.new_name).await {
        Ok(()) => {
            // Load the new agent from state
            let sw = crate::state::StateManager::new(&state.data_dir).load().await;
            match sw {
                Ok(sw) => {
                    match sw.agents.get(&req.new_name) {
                        Some(agent) => Json(ApiResponse::success(agent.clone())),
                        None => Json(ApiResponse::error("Agent not found after clone".to_string())),
                    }
                }
                Err(e) => Json(ApiResponse::error(e.to_string())),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn list_all_checkpoints(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<CheckpointMeta>>> {
    let checkpoint_mgr = crate::checkpoint::CheckpointManager::new(
        &state.data_dir,
        "/dev/sda2",
        std::path::Path::new("/home"),
    );
    
    match checkpoint_mgr.list_checkpoints(None).await {
        Ok(checkpoints) => Json(ApiResponse::success(checkpoints)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}