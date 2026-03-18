use crate::models::{CreateToolRequest, InvokeToolRequest, InvokeToolResponse, Tool, ToolMetadata, AuthorizeRequest, SetEnvRequest};
use crate::AppState;
use super::types::ApiResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;

#[derive(Debug, serde::Serialize)]
pub struct ToolInfo {
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
    pub updated_at: String,
}

impl From<Tool> for ToolInfo {
    fn from(tool: Tool) -> Self {
        ToolInfo {
            slug: tool.slug,
            name: tool.name,
            version: tool.version,
            description: tool.description,
            forgejo_repo: tool.forgejo_repo,
            git_commit: tool.git_commit,
            entrypoint: tool.entrypoint,
            input_schema: tool.input_schema,
            output_schema: tool.output_schema,
            author_agent: tool.author_agent,
            allowed_agents: tool.allowed_agents,
            enabled: tool.enabled,
            created_at: tool.created_at,
            updated_at: tool.updated_at,
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tools", get(list_tools).post(clone_tool))
        .route("/tools/{slug}", get(get_tool).delete(delete_tool))
        .route("/tools/{slug}/pull", post(pull_tool))
        .route("/tools/{slug}/authorize", post(authorize_agent))
        .route("/tools/{slug}/revoke", post(revoke_agent))
        .route("/tools/{slug}/invoke", post(invoke_tool))
        .route("/tools/{slug}/env", get(list_env).post(set_env))
        .route("/tools/{slug}/env/{key}", delete(delete_env))
}

pub async fn list_tools(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<ToolInfo>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let tools: Vec<ToolInfo> = sw.tools.values().cloned().map(ToolInfo::from).collect();
    Json(ApiResponse::success(tools))
}

pub async fn get_tool(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<ToolInfo>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.tools.get(&slug) {
        Some(tool) => Json(ApiResponse::success(ToolInfo::from(tool.clone()))),
        None => Json(ApiResponse::error(format!("Tool '{}' not found", slug))),
    }
}

pub async fn clone_tool(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateToolRequest>,
) -> Json<ApiResponse<ToolInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.tools.contains_key(&req.slug) {
        return Json(ApiResponse::error(format!("Tool '{}' already exists", req.slug)));
    }
    
    if !sw.agents.contains_key(&req.author_agent) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.author_agent)));
    }
    
    if let Err(e) = state.tool_manager.clone_tool(&req.slug, &req.forgejo_repo).await {
        return Json(ApiResponse::error(format!("Failed to clone tool: {}", e)));
    }
    
    let metadata = match state.tool_manager.parse_tool_md(&req.slug) {
        Ok(m) => m,
        Err(e) => {
            let _ = state.tool_manager.delete_tool(&req.slug).await;
            return Json(ApiResponse::error(format!("Failed to parse TOOL.md: {}", e)));
        }
    };
    
    let git_commit = match state.tool_manager.get_git_commit(&req.slug).await {
        Ok(c) => c,
        Err(e) => {
            let _ = state.tool_manager.delete_tool(&req.slug).await;
            return Json(ApiResponse::error(format!("Failed to get git commit: {}", e)));
        }
    };
    
    let now = chrono::Utc::now().to_rfc3339();
    
    let tool = Tool {
        slug: req.slug.clone(),
        name: metadata.name,
        version: metadata.version,
        description: metadata.description,
        forgejo_repo: req.forgejo_repo,
        git_commit,
        entrypoint: metadata.entrypoint,
        input_schema: metadata.input_schema,
        output_schema: metadata.output_schema,
        author_agent: req.author_agent.clone(),
        allowed_agents: vec![req.author_agent],
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.tools.insert(req.slug.clone(), tool.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        let _ = state.tool_manager.delete_tool(&req.slug).await;
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(ToolInfo::from(tool)))
}

pub async fn pull_tool(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<ToolInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.tools.contains_key(&slug) {
        return Json(ApiResponse::error(format!("Tool '{}' not found", slug)));
    }
    
    let new_commit = match state.tool_manager.pull_tool(&slug).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::error(format!("Failed to pull tool: {}", e))),
    };
    
    let metadata = match state.tool_manager.parse_tool_md(&slug) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::error(format!("Failed to parse TOOL.md: {}", e))),
    };
    
    if let Some(t) = sw.tools.get_mut(&slug) {
        t.git_commit = new_commit;
        t.version = metadata.version;
        t.description = metadata.description;
        t.entrypoint = metadata.entrypoint;
        t.input_schema = metadata.input_schema;
        t.output_schema = metadata.output_schema;
        t.updated_at = chrono::Utc::now().to_rfc3339();
        
        let tool = t.clone();
        let _ = state.state_manager.save(&sw).await;
        return Json(ApiResponse::success(ToolInfo::from(tool)));
    }
    
    Json(ApiResponse::error(format!("Tool '{}' not found", slug)))
}

pub async fn delete_tool(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.tools.remove(&slug).is_none() {
        return Json(ApiResponse::error(format!("Tool '{}' not found", slug)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let _ = state.tool_manager.delete_tool(&slug).await;
    
    Json(ApiResponse::success(format!("Tool '{}' deleted", slug)))
}

pub async fn authorize_agent(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.agents.contains_key(&req.agent_name) {
        return Json(ApiResponse::error(format!("Agent '{}' not found", req.agent_name)));
    }
    
    match sw.tools.get_mut(&slug) {
        Some(tool) => {
            if !tool.allowed_agents.contains(&req.agent_name) {
                tool.allowed_agents.push(req.agent_name.clone());
                tool.updated_at = chrono::Utc::now().to_rfc3339();
            }
            let tool_slug = tool.slug.clone();
            if let Err(e) = state.state_manager.save(&sw).await {
                return Json(ApiResponse::error(e.to_string()));
            }
            Json(ApiResponse::success(format!("Agent '{}' authorized for tool '{}'", req.agent_name, tool_slug)))
        }
        None => Json(ApiResponse::error(format!("Tool '{}' not found", slug))),
    }
}

pub async fn revoke_agent(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<AuthorizeRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.tools.get_mut(&slug) {
        Some(tool) => {
            tool.allowed_agents.retain(|a| a != &req.agent_name);
            tool.updated_at = chrono::Utc::now().to_rfc3339();
            let tool_slug = tool.slug.clone();
            if let Err(e) = state.state_manager.save(&sw).await {
                return Json(ApiResponse::error(e.to_string()));
            }
            Json(ApiResponse::success(format!("Agent '{}' revoked from tool '{}'", req.agent_name, tool_slug)))
        }
        None => Json(ApiResponse::error(format!("Tool '{}' not found", slug))),
    }
}

pub async fn invoke_tool(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<InvokeToolRequest>,
) -> Json<InvokeToolResponse> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(InvokeToolResponse {
            success: false,
            output: None,
            error: Some(e.to_string()),
        }),
    };
    
    let tool = match sw.tools.get(&slug) {
        Some(t) => t.clone(),
        None => return Json(InvokeToolResponse {
            success: false,
            output: None,
            error: Some(format!("Tool '{}' not found", slug)),
        }),
    };
    
    if let Some(caller) = &req.caller_agent {
        if !state.tool_manager.check_authorization(&tool, caller) {
            return Json(InvokeToolResponse {
                success: false,
                output: None,
                error: Some(format!("Agent '{}' is not authorized to invoke this tool", caller)),
            });
        }
    }
    
    if !tool.enabled {
        return Json(InvokeToolResponse {
            success: false,
            output: None,
            error: Some("Tool is disabled".to_string()),
        });
    }
    
    match state.tool_manager.invoke_host_tool(&tool, &req.input).await {
        Ok(response) => Json(response),
        Err(e) => Json(InvokeToolResponse {
            success: false,
            output: None,
            error: Some(e.to_string()),
        }),
    }
}

pub async fn list_env(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<Vec<String>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.tools.contains_key(&slug) {
        return Json(ApiResponse::error(format!("Tool '{}' not found", slug)));
    }
    
    match state.tool_manager.list_env(&slug).await {
        Ok(keys) => Json(ApiResponse::success(keys)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub async fn set_env(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(req): Json<SetEnvRequest>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.tools.contains_key(&slug) {
        return Json(ApiResponse::error(format!("Tool '{}' not found", slug)));
    }
    
    if let Err(e) = state.tool_manager.set_env(&slug, &req.key, &req.value).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Env '{}' set", req.key)))
}

pub async fn delete_env(
    State(state): State<Arc<AppState>>,
    Path((slug, key)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.tools.contains_key(&slug) {
        return Json(ApiResponse::error(format!("Tool '{}' not found", slug)));
    }
    
    if let Err(e) = state.tool_manager.delete_env(&slug, &key).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Env '{}' deleted", key)))
}