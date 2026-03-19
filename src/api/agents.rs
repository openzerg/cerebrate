use std::sync::Arc;
use axum::{
    Json, extract::{Path, State},
};
use crate::AppState;
use super::types::{ApiResponse, AgentInfo, CreateAgentRequest, StatsSummary, BindModelRequest};

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
        let model_name = agent.model_id.as_ref()
            .and_then(|id| sw.models.get(id))
            .map(|m| m.name.clone());
        AgentInfo {
            name: name.clone(),
            enabled: agent.enabled,
            container_ip: agent.container_ip.clone(),
            host_ip: agent.host_ip.clone(),
            forgejo_username: agent.forgejo_username.clone(),
            online,
            model_id: agent.model_id.clone(),
            model_name,
            internal_token: agent.internal_token.clone(),
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
            let model_name = agent.model_id.as_ref()
                .and_then(|id| sw.models.get(id))
                .map(|m| m.name.clone());
            Json(ApiResponse::success(AgentInfo {
                name: name.clone(),
                enabled: agent.enabled,
                container_ip: agent.container_ip.clone(),
                host_ip: agent.host_ip.clone(),
                forgejo_username: agent.forgejo_username.clone(),
                online,
                model_id: agent.model_id.clone(),
                model_name,
                internal_token: agent.internal_token.clone(),
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
    let internal_token = uuid::Uuid::new_v4().to_string();
    
    let agent = crate::models::Agent {
        enabled: true,
        container_ip: format!("{}.{}.2", sw.defaults.container_subnet_base, agent_num),
        host_ip: format!("{}.{}.1", sw.defaults.container_subnet_base, agent_num),
        forgejo_username: req.forgejo_username.clone(),
        internal_token: internal_token.clone(),
        model_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.agents.insert(req.name.clone(), agent);
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let _ = state.apply_tx.send(());
    
    Json(ApiResponse::success(AgentInfo {
        name: req.name,
        enabled: true,
        container_ip: format!("{}.{}.2", sw.defaults.container_subnet_base, agent_num),
        host_ip: format!("{}.{}.1", sw.defaults.container_subnet_base, agent_num),
        forgejo_username: req.forgejo_username,
        online: false,
        model_id: None,
        model_name: None,
        internal_token,
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
    
    let _ = state.apply_tx.send(());
    
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
    
    let _ = state.apply_tx.send(());
    
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
    
    let _ = state.apply_tx.send(());
    
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

pub async fn bind_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<BindModelRequest>,
) -> Json<ApiResponse<AgentInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if !sw.models.contains_key(&req.model_id) {
        return Json(ApiResponse::error(format!("Model '{}' not found", req.model_id)));
    }
    
    let model_name = sw.models.get(&req.model_id).map(|m| m.name.clone());
    
    let (enabled, container_ip, host_ip, forgejo_username, internal_token) = {
        let agent = match sw.agents.get_mut(&name) {
            Some(a) => a,
            None => return Json(ApiResponse::error(format!("Agent '{}' not found", name))),
        };
        
        agent.model_id = Some(req.model_id.clone());
        agent.updated_at = chrono::Utc::now().to_rfc3339();
        
        (agent.enabled, agent.container_ip.clone(), agent.host_ip.clone(), agent.forgejo_username.clone(), agent.internal_token.clone())
    };
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let online = state.vm_connections.read().await
        .get(&name).map(|c| c.connected).unwrap_or(false);
    
    Json(ApiResponse::success(AgentInfo {
        name,
        enabled,
        container_ip,
        host_ip,
        forgejo_username,
        online,
        model_id: Some(req.model_id),
        model_name,
        internal_token,
    }))
}

pub async fn unbind_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AgentInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let (enabled, container_ip, host_ip, forgejo_username, internal_token) = {
        let agent = match sw.agents.get_mut(&name) {
            Some(a) => a,
            None => return Json(ApiResponse::error(format!("Agent '{}' not found", name))),
        };
        
        agent.model_id = None;
        agent.updated_at = chrono::Utc::now().to_rfc3339();
        
        (agent.enabled, agent.container_ip.clone(), agent.host_ip.clone(), agent.forgejo_username.clone(), agent.internal_token.clone())
    };
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    let online = state.vm_connections.read().await
        .get(&name).map(|c| c.connected).unwrap_or(false);
    
    Json(ApiResponse::success(AgentInfo {
        name,
        enabled,
        container_ip,
        host_ip,
        forgejo_username,
        online,
        model_id: None,
        model_name: None,
        internal_token,
    }))
}