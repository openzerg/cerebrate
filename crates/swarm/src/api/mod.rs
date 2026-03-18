mod types;
mod websocket;
mod agents;
mod providers;
mod checkpoints;
mod skills;
mod tools;

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    routing::{get, post, delete},
    Router, middleware, Json,
};
use tower_http::cors::{CorsLayer, Any};
use serde::Serialize;

pub use types::*;
use crate::AppState;
use crate::auth::{AuthConfig, auth_middleware};

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
    auth_config: AuthConfig,
) -> crate::Result<()> {
    let auth_state = Arc::new(auth_config);
    
    let api_routes = Router::new()
        .route("/agents", get(agents::list_agents).post(agents::create_agent))
        .route("/agents/{name}", get(agents::get_agent).delete(agents::delete_agent))
        .route("/agents/{name}/enable", post(agents::enable_agent))
        .route("/agents/{name}/disable", post(agents::disable_agent))
        .route("/agents/{name}/checkpoint", post(checkpoints::create_checkpoint))
        .route("/agents/{name}/checkpoints", get(checkpoints::list_checkpoints))
        .route("/agents/{name}/rollback", post(checkpoints::rollback_agent))
        .route("/checkpoints", get(checkpoints::list_all_checkpoints))
        .route("/checkpoints/{id}", delete(checkpoints::delete_checkpoint))
        .route("/checkpoints/{id}/clone", post(checkpoints::clone_checkpoint))
        .route("/stats/summary", get(agents::get_stats_summary))
        .route("/llm/providers", get(providers::list_providers).post(providers::create_provider))
        .route("/llm/providers/{id}", delete(providers::delete_provider))
        .route("/llm/providers/{id}/enable", post(providers::enable_provider))
        .route("/llm/providers/{id}/disable", post(providers::disable_provider))
        .route("/llm/keys", get(providers::list_api_keys).post(providers::create_api_key))
        .route("/llm/keys/{id}", delete(providers::delete_api_key))
        .merge(skills::router())
        .merge(tools::router())
        .route_layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(websocket::event_ws_handler))
        .route("/ws/vm", get(websocket::vm_ws_handler))
        .nest("/api", api_routes)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Zerg Swarm listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}