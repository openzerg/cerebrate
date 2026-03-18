use crate::models::{CreateSkillRequest, InvokeSkillRequest, InvokeSkillResponse, Skill, SkillType};
use crate::AppState;
use super::types::ApiResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetSecretRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFileRequest {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    pub agent_name: String,
}

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub enabled: bool,
    pub owner_agent: String,
    pub allowed_agents: Vec<String>,
    pub entrypoint: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Skill> for SkillInfo {
    fn from(skill: Skill) -> Self {
        SkillInfo {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            skill_type: skill.skill_type.as_str().to_string(),
            enabled: skill.enabled,
            owner_agent: skill.owner_agent,
            allowed_agents: skill.allowed_agents,
            entrypoint: skill.entrypoint,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/skills", get(list_skills).post(create_skill))
        .route("/skills/{id}", get(get_skill).delete(delete_skill))
        .route("/skills/{id}/secrets", get(list_secrets).post(set_secret))
        .route("/skills/{id}/secrets/{key}", delete(delete_secret))
        .route("/skills/{id}/authorize", post(authorize_agent))
        .route("/skills/{id}/revoke", post(revoke_agent))
        .route("/skills/{id}/files", post(upload_file))
        .route("/skills/{id}/invoke", post(invoke_skill))
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
    Path(id): Path<String>,
) -> Json<ApiResponse<SkillInfo>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.skills.get(&id) {
        Some(skill) => Json(ApiResponse::success(SkillInfo::from(skill.clone()))),
        None => Json(ApiResponse::error(format!("Skill '{}' not found", id))),
    }
}

pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSkillRequest>,
) -> Json<ApiResponse<SkillInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.agents.contains_key(&req.owner_agent) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.owner_agent)));
    }
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let skill = Skill {
        id: id.clone(),
        name: req.name,
        description: String::new(),
        skill_type: req.skill_type,
        enabled: true,
        owner_agent: req.owner_agent.clone(),
        allowed_agents: vec![req.owner_agent],
        entrypoint: req.entrypoint,
        input_schema: req.input_schema,
        output_schema: req.output_schema,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.skills.insert(id.clone(), skill.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    if let Err(e) = state.skill_manager.ensure_directories().await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(SkillInfo::from(skill)))
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.skills.remove(&id).is_none() {
        return Json(ApiResponse::error(format!("Skill '{}' not found", id)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let _ = state.skill_manager.delete_skill_files(&id).await;
    let _ = state.skill_manager.delete_all_secrets(&id).await;
    
    Json(ApiResponse::success(format!("Skill '{}' deleted", id)))
}

pub async fn set_secret(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SetSecretRequest>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&id) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", id)));
    }
    
    if let Err(e) = state.skill_manager.set_secret(&id, &req.key, &req.value).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Secret '{}' set", req.key)))
}

pub async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<String>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&id) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", id)));
    }
    
    match state.skill_manager.list_secrets(&id).await {
        Ok(secrets) => Json(ApiResponse::success(secrets)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&id) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", id)));
    }
    
    if let Err(e) = state.skill_manager.delete_secret(&id, &key).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Secret '{}' deleted", key)))
}

pub async fn authorize_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.agents.contains_key(&req.agent_name) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.agent_name)));
    }
    
    match sw.skills.get_mut(&id) {
        Some(skill) => {
            let skill_name = skill.name.clone();
            if !skill.allowed_agents.contains(&req.agent_name) {
                skill.allowed_agents.push(req.agent_name.clone());
                skill.updated_at = chrono::Utc::now().to_rfc3339();
                if let Err(e) = state.state_manager.save(&sw).await {
                    return Json(ApiResponse::error(e.to_string()));
                }
            }
            Json(ApiResponse::success(format!("Agent '{}' authorized for skill '{}'", req.agent_name, skill_name)))
        }
        None => Json(ApiResponse::error(format!("Skill '{}' not found", id))),
    }
}

pub async fn revoke_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.skills.get_mut(&id) {
        Some(skill) => {
            skill.allowed_agents.retain(|a| a != &req.agent_name);
            skill.updated_at = chrono::Utc::now().to_rfc3339();
            let skill_name = skill.name.clone();
            if let Err(e) = state.state_manager.save(&sw).await {
                return Json(ApiResponse::error(e.to_string()));
            }
            Json(ApiResponse::success(format!("Agent '{}' revoked from skill '{}'", req.agent_name, skill_name)))
        }
        None => Json(ApiResponse::error(format!("Skill '{}' not found", id))),
    }
}

pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UploadFileRequest>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&id) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", id)));
    }
    
    if let Err(e) = state.skill_manager.write_skill_file(&id, &req.filename, &req.content).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("File '{}' uploaded", req.filename)))
}

pub async fn invoke_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<InvokeSkillRequest>,
) -> Json<InvokeSkillResponse> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(InvokeSkillResponse {
            success: false,
            output: None,
            error: Some(e.to_string()),
        }),
    };
    
    let skill = match sw.skills.get(&id) {
        Some(s) => s.clone(),
        None => return Json(InvokeSkillResponse {
            success: false,
            output: None,
            error: Some(format!("Skill '{}' not found", id)),
        }),
    };
    
    if !state.skill_manager.check_authorization(&skill, &req.caller_agent) {
        return Json(InvokeSkillResponse {
            success: false,
            output: None,
            error: Some(format!("Agent '{}' is not authorized to invoke this skill", req.caller_agent)),
        });
    }
    
    if !skill.enabled {
        return Json(InvokeSkillResponse {
            success: false,
            output: None,
            error: Some("Skill is disabled".to_string()),
        });
    }
    
    match skill.skill_type {
        SkillType::HostScript => {
            match state.skill_manager.invoke_host_script(&skill, &req.input).await {
                Ok(response) => Json(response),
                Err(e) => Json(InvokeSkillResponse {
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        SkillType::AgentScript => {
            invoke_agent_script(state, skill, req).await
        }
    }
}

async fn invoke_agent_script(
    state: Arc<AppState>,
    skill: Skill,
    req: InvokeSkillRequest,
) -> Json<InvokeSkillResponse> {
    Json(InvokeSkillResponse {
        success: false,
        output: None,
        error: Some("AgentScript execution requires openzerg agent support. Please ensure the agent is running with skill execution enabled.".to_string()),
    })
}