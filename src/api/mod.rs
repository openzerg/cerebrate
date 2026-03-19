pub mod types;
mod websocket;
mod agents;
mod providers;
mod checkpoints;
pub mod skills;
pub mod tools;
mod agent;
mod rpc;

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    routing::{get, post, delete, put},
    Router, middleware, Json,
};
use tower_http::cors::{CorsLayer, Any};
use serde::Serialize;

pub use types::*;
use crate::AppState;
use crate::auth::{AuthState, auth_middleware};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    name: &'static str,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: VERSION,
        name: "zerg-swarm",
    })
}

pub async fn start_server(
    addr: SocketAddr, 
    state: Arc<AppState>,
) -> crate::Result<()> {
    let auth_state = Arc::new(AuthState {
        state_manager: Arc::new(state.state_manager.clone()),
    });
    
    let api_routes = Router::new()
        .route("/agents", get(agents::list_agents).post(agents::create_agent))
        .route("/agents/{name}", get(agents::get_agent).delete(agents::delete_agent))
        .route("/agents/{name}/enable", post(agents::enable_agent))
        .route("/agents/{name}/disable", post(agents::disable_agent))
        .route("/agents/{name}/bind-model", post(agents::bind_model))
        .route("/agents/{name}/unbind-model", post(agents::unbind_model))
        .route("/agents/{name}/checkpoint", post(checkpoints::create_checkpoint))
        .route("/agents/{name}/checkpoints", get(checkpoints::list_checkpoints))
        .route("/agents/{name}/rollback", post(checkpoints::rollback_agent))
        .route("/agents/{name}/query", post(agent::query_agent))
        .route("/agents/{name}/events", post(agent::send_event))
        .route("/agents/{name}/files", get(agent::list_files))
        .route("/agents/{name}/files/{*path}", get(agent::get_file).put(agent::update_file))
        .route("/agents/{name}/sessions", get(agent::proxy_sessions))
        .route("/agents/{name}/sessions/{id}", get(agent::proxy_session))
        .route("/agents/{name}/sessions/{id}/messages", get(agent::proxy_session_messages))
        .route("/agents/{name}/sessions/{id}/chat", post(agent::proxy_session_chat))
        .route("/agents/{name}/sessions/{id}/interrupt", post(agent::proxy_session_interrupt))
        .route("/agents/{name}/sessions/{id}/context", get(agent::proxy_session_context))
        .route("/agents/{name}/processes", get(agent::proxy_processes))
        .route("/agents/{name}/processes/{id}", get(agent::proxy_process))
        .route("/agents/{name}/processes/{id}/output", get(agent::proxy_process_output))
        .route("/agents/{name}/tasks", get(agent::proxy_tasks))
        .route("/agents/{name}/activities", get(agent::proxy_activities))
        .route("/agents/{name}/message", post(agent::proxy_message))
        .route("/agents/{name}/remind", post(agent::proxy_remind))
        .route("/checkpoints", get(checkpoints::list_all_checkpoints))
        .route("/checkpoints/{id}", delete(checkpoints::delete_checkpoint))
        .route("/checkpoints/{id}/clone", post(checkpoints::clone_checkpoint))
        .route("/stats/summary", get(agents::get_stats_summary))
        .route("/llm/providers", get(providers::list_providers).post(providers::create_provider))
        .route("/llm/providers/{id}", delete(providers::delete_provider))
        .route("/llm/providers/{id}/enable", post(providers::enable_provider))
        .route("/llm/providers/{id}/disable", post(providers::disable_provider))
        .route("/llm/models", get(providers::list_models).post(providers::create_model))
        .route("/llm/models/{id}", delete(providers::delete_model))
        .route("/llm/models/{id}/enable", post(providers::enable_model))
        .route("/llm/models/{id}/disable", post(providers::disable_model))
        .merge(skills::router())
        .merge(tools::router())
        .route_layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(websocket::event_ws_handler))
        .route("/ws/vm", get(websocket::vm_ws_handler))
        .route("/rpc", get(rpc::rpc_ws_handler))
        .nest("/api", api_routes)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Zerg Swarm listening on {}", addr);
    tracing::info!("RPC endpoint available at ws://{}/rpc", addr);
    axum::serve(listener, app).await?;

    Ok(())
}