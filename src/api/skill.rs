use axum::{extract::{State, Path}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub author_agent: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneRequest {
    pub author_agent: String,
    pub forgejo_repo: String,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Skill>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let skills: Vec<Skill> = sw.skills.iter().map(|(_, s)| Skill {
        slug: s.slug.clone(),
        name: s.name.clone(),
        version: s.version.clone(),
        description: s.description.clone(),
        forgejo_repo: s.forgejo_repo.clone(),
        git_commit: s.git_commit.clone(),
        author_agent: s.author_agent.clone(),
        created_at: s.created_at.clone(),
    }).collect();
    
    Json(ApiResponse::ok(skills))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<Skill>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let skill = match sw.skills.get(&slug) {
        Some(s) => s,
        None => return Json(ApiResponse::err(&format!("Skill {} not found", slug))),
    };
    
    Json(ApiResponse::ok(Skill {
        slug: skill.slug.clone(),
        name: skill.name.clone(),
        version: skill.version.clone(),
        description: skill.description.clone(),
        forgejo_repo: skill.forgejo_repo.clone(),
        git_commit: skill.git_commit.clone(),
        author_agent: skill.author_agent.clone(),
        created_at: skill.created_at.clone(),
    }))
}

pub async fn clone(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(_req): Json<CloneRequest>,
) -> Json<ApiResponse<Skill>> {
    Json(ApiResponse::err("Not implemented - use gRPC"))
}

pub async fn pull(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<Skill>> {
    Json(ApiResponse::err("Not implemented - use gRPC"))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if sw.skills.remove(&slug).is_none() {
        return Json(ApiResponse::err(&format!("Skill {} not found", slug)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}