use axum::{extract::{State, Path}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub forgejo_repo: String,
    pub git_commit: String,
    pub entrypoint: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub author_agent: String,
    pub allowed_agents: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub agent_name: String,
}

#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    pub input: serde_json::Value,
    pub caller: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetEnvRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct InvokeResponse {
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Tool>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let tools: Vec<Tool> = sw.tools.iter().map(|(_, t)| Tool {
        slug: t.slug.clone(),
        name: t.name.clone(),
        version: t.version.clone(),
        description: t.description.clone(),
        forgejo_repo: t.forgejo_repo.clone(),
        git_commit: t.git_commit.clone(),
        entrypoint: t.entrypoint.clone(),
        input_schema: t.input_schema.clone(),
        output_schema: t.output_schema.clone(),
        author_agent: t.author_agent.clone(),
        allowed_agents: t.allowed_agents.clone(),
        enabled: t.enabled,
        created_at: t.created_at.clone(),
    }).collect();
    
    Json(ApiResponse::ok(tools))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<Tool>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let tool = match sw.tools.get(&slug) {
        Some(t) => t,
        None => return Json(ApiResponse::err(&format!("Tool {} not found", slug))),
    };
    
    Json(ApiResponse::ok(Tool {
        slug: tool.slug.clone(),
        name: tool.name.clone(),
        version: tool.version.clone(),
        description: tool.description.clone(),
        forgejo_repo: tool.forgejo_repo.clone(),
        git_commit: tool.git_commit.clone(),
        entrypoint: tool.entrypoint.clone(),
        input_schema: tool.input_schema.clone(),
        output_schema: tool.output_schema.clone(),
        author_agent: tool.author_agent.clone(),
        allowed_agents: tool.allowed_agents.clone(),
        enabled: tool.enabled,
        created_at: tool.created_at.clone(),
    }))
}

pub async fn clone(
    State(state): State<Arc<AppState>>,
    Path(_slug): Path<String>,
    Json(_req): Json<super::skill::CloneRequest>,
) -> Json<ApiResponse<Tool>> {
    Json(ApiResponse::err("Not implemented - use gRPC"))
}

pub async fn pull(
    State(state): State<Arc<AppState>>,
    Path(_slug): Path<String>,
) -> Json<ApiResponse<Tool>> {
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
    
    if sw.tools.remove(&slug).is_none() {
        return Json(ApiResponse::err(&format!("Tool {} not found", slug)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn authorize(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let tool = match sw.tools.get_mut(&slug) {
        Some(t) => t,
        None => return Json(ApiResponse::err(&format!("Tool {} not found", slug))),
    };
    
    if !tool.allowed_agents.contains(&req.agent_name) {
        tool.allowed_agents.push(req.agent_name);
        tool.updated_at = chrono::Utc::now().to_rfc3339();
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn revoke(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let tool = match sw.tools.get_mut(&slug) {
        Some(t) => t,
        None => return Json(ApiResponse::err(&format!("Tool {} not found", slug))),
    };
    
    tool.allowed_agents.retain(|a| a != &req.agent_name);
    tool.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn invoke(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<InvokeRequest>,
) -> Json<ApiResponse<InvokeResponse>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let tool = match sw.tools.get(&slug) {
        Some(t) => t,
        None => return Json(ApiResponse::err(&format!("Tool {} not found", slug))),
    };
    
    if let Some(caller) = &req.caller {
        if !state.tool_manager.check_authorization(tool, caller) {
            return Json(ApiResponse::err(&format!("Agent {} is not authorized to use tool {}", caller, slug)));
        }
    }
    
    let result = state.tool_manager.invoke_host_tool(tool, &req.input).await;
    
    match result {
        Ok(resp) => Json(ApiResponse::ok(InvokeResponse {
            output: resp.output,
            error: resp.error,
        })),
        Err(e) => Json(ApiResponse::ok(InvokeResponse {
            output: None,
            error: Some(e.to_string()),
        })),
    }
}

pub async fn list_env(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<Vec<String>>> {
    let keys = match state.tool_manager.list_env(&slug).await {
        Ok(k) => k,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    Json(ApiResponse::ok(keys))
}

pub async fn set_env(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<SetEnvRequest>,
) -> Json<ApiResponse<()>> {
    if let Err(e) = state.tool_manager.set_env(&slug, &req.key, &req.value).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn delete_env(
    State(state): State<Arc<AppState>>,
    Path((slug, key)): Path<(String, String)>,
) -> Json<ApiResponse<()>> {
    if let Err(e) = state.tool_manager.delete_env(&slug, &key).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}