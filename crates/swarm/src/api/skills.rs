use crate::models::{CreateSkillRequest, InvokeSkillRequest, InvokeSkillResponse, Skill, SkillMetadata, SkillType};
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
pub struct AuthorizeRequest {
    pub agent_name: String,
}

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub skill_type: String,
    pub enabled: bool,
    pub author_agent: String,
    pub allowed_agents: Vec<String>,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub entrypoint: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Skill> for SkillInfo {
    fn from(skill: Skill) -> Self {
        SkillInfo {
            id: skill.id,
            name: skill.name,
            version: skill.version,
            description: skill.description,
            skill_type: skill.skill_type.as_str().to_string(),
            enabled: skill.enabled,
            author_agent: skill.author_agent,
            allowed_agents: skill.allowed_agents,
            forgejo_repo: skill.forgejo_repo,
            git_commit: skill.git_commit,
            entrypoint: skill.entrypoint,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/skills", get(list_skills).post(clone_skill))
        .route("/skills/{name}", get(get_skill).delete(delete_skill))
        .route("/skills/{name}/pull", post(pull_skill))
        .route("/skills/{name}/secrets", get(list_secrets).post(set_secret))
        .route("/skills/{name}/secrets/{key}", delete(delete_secret))
        .route("/skills/{name}/authorize", post(authorize_agent))
        .route("/skills/{name}/revoke", post(revoke_agent))
        .route("/skills/{name}/invoke", post(invoke_skill))
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
    Path(name): Path<String>,
) -> Json<ApiResponse<SkillInfo>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.skills.get(&name) {
        Some(skill) => Json(ApiResponse::success(SkillInfo::from(skill.clone()))),
        None => Json(ApiResponse::error(format!("Skill '{}' not found", name))),
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
    
    if sw.skills.contains_key(&req.name) {
        return Json(ApiResponse::error(format!("Skill '{}' already exists", req.name)));
    }
    
    if !sw.agents.contains_key(&req.author_agent) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.author_agent)));
    }
    
    if let Err(e) = state.skill_manager.clone_skill(&req.name, &req.forgejo_repo).await {
        return Json(ApiResponse::error(format!("Failed to clone skill: {}", e)));
    }
    
    let metadata = match state.skill_manager.parse_skill_md(&req.name) {
        Ok(m) => m,
        Err(e) => {
            let _ = state.skill_manager.delete_skill(&req.name).await;
            return Json(ApiResponse::error(format!("Failed to parse SKILL.md: {}", e)));
        }
    };
    
    let git_commit = match state.skill_manager.get_git_commit(&req.name).await {
        Ok(c) => c,
        Err(e) => {
            let _ = state.skill_manager.delete_skill(&req.name).await;
            return Json(ApiResponse::error(format!("Failed to get git commit: {}", e)));
        }
    };
    
    let skill_type = metadata.skill_type
        .and_then(|s| SkillType::from_str(&s))
        .unwrap_or(SkillType::HostScript);
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let skill = Skill {
        id,
        name: req.name.clone(),
        version: metadata.version,
        description: metadata.description,
        skill_type,
        enabled: true,
        author_agent: req.author_agent.clone(),
        allowed_agents: vec![req.author_agent],
        forgejo_repo: req.forgejo_repo,
        git_commit,
        entrypoint: metadata.entrypoint,
        permissions: metadata.permissions,
        input_schema: metadata.input_schema,
        output_schema: metadata.output_schema,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.skills.insert(req.name.clone(), skill.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        let _ = state.skill_manager.delete_skill(&req.name).await;
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(SkillInfo::from(skill)))
}

pub async fn pull_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<SkillInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&name) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", name)));
    }
    
    let new_commit = match state.skill_manager.pull_skill(&name).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::error(format!("Failed to pull skill: {}", e))),
    };
    
    let metadata = match state.skill_manager.parse_skill_md(&name) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::error(format!("Failed to parse SKILL.md: {}", e))),
    };
    
    if let Some(s) = sw.skills.get_mut(&name) {
        s.git_commit = new_commit;
        s.version = metadata.version;
        s.description = metadata.description;
        s.entrypoint = metadata.entrypoint;
        s.permissions = metadata.permissions;
        s.input_schema = metadata.input_schema;
        s.output_schema = metadata.output_schema;
        s.updated_at = chrono::Utc::now().to_rfc3339();
        
        let skill = s.clone();
        let _ = state.state_manager.save(&sw).await;
        return Json(ApiResponse::success(SkillInfo::from(skill)));
    }
    
    Json(ApiResponse::error(format!("Skill '{}' not found", name)))
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.skills.remove(&name).is_none() {
        return Json(ApiResponse::error(format!("Skill '{}' not found", name)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let _ = state.skill_manager.delete_skill(&name).await;
    
    Json(ApiResponse::success(format!("Skill '{}' deleted", name)))
}

pub async fn set_secret(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<SetSecretRequest>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&name) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", name)));
    }
    
    if let Err(e) = state.skill_manager.set_secret(&name, &req.key, &req.value).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Secret '{}' set", req.key)))
}

pub async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<Vec<String>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&name) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", name)));
    }
    
    match state.skill_manager.list_secrets(&name).await {
        Ok(secrets) => Json(ApiResponse::success(secrets)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Path((name, key)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.skills.contains_key(&name) {
        return Json(ApiResponse::error(format!("Skill '{}' not found", name)));
    }
    
    if let Err(e) = state.skill_manager.delete_secret(&name, &key).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Secret '{}' deleted", key)))
}

pub async fn authorize_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.agents.contains_key(&req.agent_name) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.agent_name)));
    }
    
    match sw.skills.get_mut(&name) {
        Some(skill) => {
            if !skill.allowed_agents.contains(&req.agent_name) {
                skill.allowed_agents.push(req.agent_name.clone());
                skill.updated_at = chrono::Utc::now().to_rfc3339();
            }
            let skill_name = skill.name.clone();
            if let Err(e) = state.state_manager.save(&sw).await {
                return Json(ApiResponse::error(e.to_string()));
            }
            Json(ApiResponse::success(format!("Agent '{}' authorized for skill '{}'", req.agent_name, skill_name)))
        }
        None => Json(ApiResponse::error(format!("Skill '{}' not found", name))),
    }
}

pub async fn revoke_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.skills.get_mut(&name) {
        Some(skill) => {
            skill.allowed_agents.retain(|a| a != &req.agent_name);
            skill.updated_at = chrono::Utc::now().to_rfc3339();
            let skill_name = skill.name.clone();
            if let Err(e) = state.state_manager.save(&sw).await {
                return Json(ApiResponse::error(e.to_string()));
            }
            Json(ApiResponse::success(format!("Agent '{}' revoked from skill '{}'", req.agent_name, skill_name)))
        }
        None => Json(ApiResponse::error(format!("Skill '{}' not found", name))),
    }
}

pub async fn invoke_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
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
    
    let skill = match sw.skills.get(&name) {
        Some(s) => s.clone(),
        None => return Json(InvokeSkillResponse {
            success: false,
            output: None,
            error: Some(format!("Skill '{}' not found", name)),
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
            Json(InvokeSkillResponse {
                success: false,
                output: None,
                error: Some("AgentScript execution requires agent container support".to_string()),
            })
        }
    }
}