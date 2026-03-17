use std::sync::Arc;
use axum::{
    Json, extract::{Path, State},
};
use crate::AppState;
use super::types::{ApiResponse, AgentInfo, CreateAgentRequest, StatsSummary};

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<AgentInfo>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let connections = state.vm_connections.read().await;
    
    let agents: Vec<AgentInfo> = sw.agents.iter().map(|(name, agent)| {
        let online = connections.get(name).map(|c| c.connected).unwrap_or(false);
        AgentInfo {
            name: name.clone(),
            enabled: agent.enabled,
            container_ip: agent.container_ip.clone(),
            host_ip: agent.host_ip.clone(),
            forgejo_username: agent.forgejo_username.clone(),
            online,
        }
    }).collect();
    
    Json(ApiResponse::success(agents))
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AgentInfo>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.agents.get(&name) {
        Some(agent) => {
            let online = state.vm_connections.read().await
                .get(&name).map(|c| c.connected).unwrap_or(false);
            Json(ApiResponse::success(AgentInfo {
                name: name.clone(),
                enabled: agent.enabled,
                container_ip: agent.container_ip.clone(),
                host_ip: agent.host_ip.clone(),
                forgejo_username: agent.forgejo_username.clone(),
                online,
            }))
        }
        None => Json(ApiResponse::error(format!("Agent '{}' not found", name))),
    }
}

pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Json<ApiResponse<AgentInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.agents.contains_key(&req.name) {
        return Json(ApiResponse::error(format!("Agent '{}' already exists", req.name)));
    }
    
    let agent_num = sw.agents.len() + 1;
    let now = chrono::Utc::now().to_rfc3339();
    
    let agent = crate::models::Agent {
        enabled: true,
        container_ip: format!("{}.{}.2", sw.defaults.container_subnet_base, agent_num),
        host_ip: format!("{}.{}.1", sw.defaults.container_subnet_base, agent_num),
        forgejo_username: req.forgejo_username.clone(),
        internal_token: uuid::Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.agents.insert(req.name.clone(), agent.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(AgentInfo {
        name: req.name,
        enabled: true,
        container_ip: agent.container_ip,
        host_ip: agent.host_ip,
        forgejo_username: agent.forgejo_username,
        online: false,
    }))
}

pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.agents.remove(&name).is_none() {
        return Json(ApiResponse::error(format!("Agent '{}' not found", name)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Agent '{}' deleted", name)))
}

pub async fn enable_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.agents.get_mut(&name) {
        Some(agent) => {
            agent.enabled = true;
            agent.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Agent '{}' not found", name))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Agent '{}' enabled", name)))
}

pub async fn disable_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.agents.get_mut(&name) {
        Some(agent) => {
            agent.enabled = false;
            agent.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Agent '{}' not found", name))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Agent '{}' disabled", name)))
}

pub async fn get_stats_summary(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<StatsSummary>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let connections = state.vm_connections.read().await;
    let online_agents = connections.values().filter(|c| c.connected).count();
    
    Json(ApiResponse::success(StatsSummary {
        total_agents: sw.agents.len(),
        online_agents,
        enabled_agents: sw.agents.values().filter(|a| a.enabled).count(),
    }))
}