use crate::models::{CreateSkillRequest, Skill, SkillMetadata};
use crate::AppState;
use super::types::ApiResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub author_agent: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Skill> for SkillInfo {
    fn from(skill: Skill) -> Self {
        SkillInfo {
            slug: skill.slug,
            name: skill.name,
            version: skill.version,
            description: skill.description,
            forgejo_repo: skill.forgejo_repo,
            git_commit: skill.git_commit,
            author_agent: skill.author_agent,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/skills", get(list_skills).post(clone_skill))
        .route("/skills/{slug}", get(get_skill).delete(delete_skill))
        .route("/skills/{slug}/pull", post(pull_skill))
}

pub async fn list_skills(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<SkillInfo>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let skills: Vec<SkillInfo> = sw.skills.values().cloned().map(SkillInfo::from).collect();
    Json(ApiResponse::success(skills))
}

pub async fn get_skill(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<SkillInfo>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.skills.get(&slug) {
        Some(skill) => Json(ApiResponse::success(SkillInfo::from(skill.clone()))),
        None => Json(ApiResponse::error(format!("Skill '{}' not found", slug))),
    }
}

pub async fn clone_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSkillRequest>,
) -> Json<ApiResponse<SkillInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.skills.contains_key(&req.slug) {
        return Json(ApiResponse::error(format!("Skill '{}' already exists", req.slug)));
    }
    
    if !sw.agents.contains_key(&req.author_agent) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.author_agent)));
    }
    
    if let Err(e) = state.tool_manager.clone_skill(&req.slug, &req.forgejo_repo).await {
        return Json(ApiResponse::error(format!("Failed to clone skill: {}", e)));
    }
    
    let metadata = match state.tool_manager.parse_skill_md(&req.slug) {
        Ok(m) => m,
        Err(e) => {
            let _ = state.tool_manager.delete_skill(&req.slug).await;
            return Json(ApiResponse::error(format!("Failed to parse SKILL.md: {}", e)));
        }
    };
    
    let git_commit = match state.tool_manager.get_skill_git_commit(&req.slug).await {
        Ok(c) => c,
        Err(e) => {
            let _ = state.tool_manager.delete_skill(&req.slug).await;
            return Json(ApiResponse::error(format!("Failed to get git commit: {}", e)));
        }
    };
    
    let now = chrono::Utc::now().to_rfc3339();
    
    let skill = Skill {
        slug: req.slug.clone(),
        name: metadata.name,
        version: metadata.version,
        description: metadata.description,
        forgejo_repo: req.forgejo_repo,
        git_commit,
        author_agent: req.author_agent,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.skills.insert(req.slug.clone(), skill.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        let _ = state.tool_manager.delete_skill(&req.slug).await;
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(SkillInfo::from(skill)))
}

pub async fn pull_skill(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<SkillInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&slug) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", slug)));
    }
    
    let new_commit = match state.tool_manager.pull_skill(&slug).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::error(format!("Failed to pull skill: {}", e))),
    };
    
    let metadata = match state.tool_manager.parse_skill_md(&slug) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::error(format!("Failed to parse SKILL.md: {}", e))),
    };
    
    if let Some(s) = sw.skills.get_mut(&slug) {
        s.git_commit = new_commit;
        s.version = metadata.version;
        s.description = metadata.description;
        s.updated_at = chrono::Utc::now().to_rfc3339();
        
        let skill = s.clone();
        let _ = state.state_manager.save(&sw).await;
        return Json(ApiResponse::success(SkillInfo::from(skill)));
    }
    
    Json(ApiResponse::error(format!("Skill '{}' not found", slug)))
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.skills.remove(&slug).is_none() {
        return Json(ApiResponse::error(format!("Skill '{}' not found", slug)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let _ = state.tool_manager.delete_skill(&slug).await;
    
    Json(ApiResponse::success(format!("Skill '{}' deleted", slug)))
}