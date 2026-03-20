use axum::{extract::{State, Path}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub enabled: bool,
    pub container_ip: String,
    pub host_ip: String,
    pub forgejo_username: Option<String>,
    pub online: bool,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub forgejo_username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BindModelRequest {
    pub model_id: String,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Agent>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let connections = state.vm_connections.read().await;
    
    let agents: Vec<Agent> = sw.agents.iter().map(|(name, a)| {
        let conn = connections.get(name);
        Agent {
            name: name.clone(),
            enabled: a.enabled,
            container_ip: a.container_ip.clone(),
            host_ip: a.host_ip.clone(),
            forgejo_username: a.forgejo_username.clone(),
            online: conn.map(|c| c.connected).unwrap_or(false),
            model_id: a.model_id.clone(),
            model_name: None,
        }
    }).collect();
    
    Json(ApiResponse::ok(agents))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<Agent>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let agent = match sw.agents.get(&name) {
        Some(a) => a,
        None => return Json(ApiResponse::err(&format!("Agent {} not found", name))),
    };
    
    let connections = state.vm_connections.read().await;
    let conn = connections.get(&name);
    
    Json(ApiResponse::ok(Agent {
        name: name.clone(),
        enabled: agent.enabled,
        container_ip: agent.container_ip.clone(),
        host_ip: agent.host_ip.clone(),
        forgejo_username: agent.forgejo_username.clone(),
        online: conn.map(|c| c.connected).unwrap_or(false),
        model_id: agent.model_id.clone(),
        model_name: None,
    }))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Json<ApiResponse<Agent>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if sw.agents.contains_key(&req.name) {
        return Json(ApiResponse::err(&format!("Agent {} already exists", req.name)));
    }
    
    let container_ip = format!("192.168.200.{}.2", sw.agents.len() + 2);
    let now = chrono::Utc::now().to_rfc3339();
    
    let agent = crate::models::Agent {
        enabled: true,
        container_ip: container_ip.clone(),
        host_ip: "192.168.200.1.1".to_string(),
        forgejo_username: req.forgejo_username.clone(),
        internal_token: uuid::Uuid::new_v4().to_string(),
        model_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.agents.insert(req.name.clone(), agent.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::ok(Agent {
        name: req.name,
        enabled: agent.enabled,
        container_ip: agent.container_ip,
        host_ip: agent.host_ip,
        forgejo_username: agent.forgejo_username,
        online: false,
        model_id: agent.model_id,
        model_name: None,
    }))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if sw.agents.remove(&name).is_none() {
        return Json(ApiResponse::err(&format!("Agent {} not found", name)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::ok(()))
}

pub async fn enable(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let agent = match sw.agents.get_mut(&name) {
        Some(a) => a,
        None => return Json(ApiResponse::err(&format!("Agent {} not found", name))),
    };
    
    agent.enabled = true;
    agent.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::ok(()))
}

pub async fn disable(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let agent = match sw.agents.get_mut(&name) {
        Some(a) => a,
        None => return Json(ApiResponse::err(&format!("Agent {} not found", name))),
    };
    
    agent.enabled = false;
    agent.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::ok(()))
}

pub async fn bind_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<BindModelRequest>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let agent = match sw.agents.get_mut(&name) {
        Some(a) => a,
        None => return Json(ApiResponse::err(&format!("Agent {} not found", name))),
    };
    
    if !sw.models.contains_key(&req.model_id) {
        return Json(ApiResponse::err(&format!("Model {} not found", req.model_id)));
    }
    
    agent.model_id = Some(req.model_id);
    agent.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn unbind_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let agent = match sw.agents.get_mut(&name) {
        Some(a) => a,
        None => return Json(ApiResponse::err(&format!("Agent {} not found", name))),
    };
    
    agent.model_id = None;
    agent.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}