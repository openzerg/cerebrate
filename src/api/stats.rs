use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_agents: usize,
    pub enabled_agents: usize,
    pub online_agents: usize,
}

pub async fn summary(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<StatsSummary>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let connections = state.vm_connections.read().await;
    let online_count = connections.values().filter(|c| c.connected).count();
    
    Json(ApiResponse::ok(StatsSummary {
        total_agents: sw.agents.len(),
        enabled_agents: sw.agents.values().filter(|a| a.enabled).count(),
        online_agents: online_count,
    }))
}