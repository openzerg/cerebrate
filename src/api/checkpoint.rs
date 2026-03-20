use axum::{extract::{State, Path}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub agent_name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckpointRequest {
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub checkpoint_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneRequest {
    pub new_name: String,
}

pub async fn list_all(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Checkpoint>>> {
    let checkpoints = match state.state_manager.list_checkpoints(None).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let result: Vec<Checkpoint> = checkpoints.into_iter().map(|c| Checkpoint {
        id: c.id,
        agent_name: c.agent_name,
        description: c.description,
        created_at: c.created_at,
    }).collect();
    
    Json(ApiResponse::ok(result))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
) -> Json<ApiResponse<Vec<Checkpoint>>> {
    let checkpoints = match state.state_manager.list_checkpoints(Some(&agent_name)).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let result: Vec<Checkpoint> = checkpoints.into_iter().map(|c| Checkpoint {
        id: c.id,
        agent_name: c.agent_name,
        description: c.description,
        created_at: c.created_at,
    }).collect();
    
    Json(ApiResponse::ok(result))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(req): Json<CreateCheckpointRequest>,
) -> Json<ApiResponse<Checkpoint>> {
    let checkpoint_manager = crate::checkpoint::CheckpointManager::new(&state.data_dir);
    
    let id = match checkpoint_manager.create_checkpoint(
        &agent_name,
        req.description.as_deref().unwrap_or(""),
    ).await {
        Ok(id) => id,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let checkpoints = match state.state_manager.list_checkpoints(Some(&agent_name)).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let checkpoint = match checkpoints.into_iter().find(|c| c.id == id) {
        Some(c) => c,
        None => return Json(ApiResponse::err("Checkpoint created but not found")),
    };
    
    Json(ApiResponse::ok(Checkpoint {
        id: checkpoint.id,
        agent_name: checkpoint.agent_name,
        description: checkpoint.description,
        created_at: checkpoint.created_at,
    }))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    if let Err(e) = state.state_manager.delete_checkpoint(&id).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn rollback(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> Json<ApiResponse<()>> {
    let checkpoint_manager = crate::checkpoint::CheckpointManager::new(&state.data_dir);
    
    if let Err(e) = checkpoint_manager.rollback(&agent_name, &req.checkpoint_id).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn clone(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CloneRequest>,
) -> Json<ApiResponse<super::agent::Agent>> {
    let checkpoint_manager = crate::checkpoint::CheckpointManager::new(&state.data_dir);
    
    if let Err(e) = checkpoint_manager.clone(&id, &req.new_name).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::err("Checkpoint cloned successfully - run 'incus copy' to create the container"))
}