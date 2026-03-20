pub mod agent;
pub mod llm;
pub mod checkpoint;
pub mod skill;
pub mod tool;
pub mod stats;
pub mod auth;

use axum::{
    routing::{get, post, delete, put},
    Router, Json,
};
use tower_http::cors::{CorsLayer, Any};
use serde::Serialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub name: &'static str,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    
    pub fn err(msg: &str) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()) }
    }
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: VERSION,
        name: "cerebrate",
    })
}

pub fn create_router(state: std::sync::Arc<crate::AppState>) -> Router<()> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/verify", post(auth::verify))
        .route("/api/agents", get(agent::list).post(agent::create))
        .route("/api/agents/{name}", get(agent::get).delete(agent::delete))
        .route("/api/agents/{name}/enable", post(agent::enable))
        .route("/api/agents/{name}/disable", post(agent::disable))
        .route("/api/agents/{name}/bind-model", post(agent::bind_model))
        .route("/api/agents/{name}/unbind-model", post(agent::unbind_model))
        .route("/api/llm/providers", get(llm::list_providers).post(llm::create_provider))
        .route("/api/llm/providers/{id}", delete(llm::delete_provider))
        .route("/api/llm/providers/{id}/enable", post(llm::enable_provider))
        .route("/api/llm/providers/{id}/disable", post(llm::disable_provider))
        .route("/api/llm/models", get(llm::list_models).post(llm::create_model))
        .route("/api/llm/models/{id}", delete(llm::delete_model))
        .route("/api/llm/models/{id}/enable", post(llm::enable_model))
        .route("/api/llm/models/{id}/disable", post(llm::disable_model))
        .route("/api/checkpoints", get(checkpoint::list_all))
        .route("/api/checkpoints/{id}", delete(checkpoint::delete))
        .route("/api/checkpoints/{id}/clone", post(checkpoint::clone))
        .route("/api/agents/{name}/checkpoints", get(checkpoint::list))
        .route("/api/agents/{name}/checkpoint", post(checkpoint::create))
        .route("/api/agents/{name}/rollback", post(checkpoint::rollback))
        .route("/api/skills", get(skill::list))
        .route("/api/skills/{slug}", get(skill::get))
        .route("/api/skills/{slug}/clone", post(skill::clone))
        .route("/api/skills/{slug}/pull", post(skill::pull))
        .route("/api/skills/{slug}", delete(skill::delete))
        .route("/api/tools", get(tool::list))
        .route("/api/tools/{slug}", get(tool::get))
        .route("/api/tools/{slug}/clone", post(tool::clone))
        .route("/api/tools/{slug}/pull", post(tool::pull))
        .route("/api/tools/{slug}", delete(tool::delete))
        .route("/api/tools/{slug}/authorize", post(tool::authorize))
        .route("/api/tools/{slug}/revoke", post(tool::revoke))
        .route("/api/tools/{slug}/invoke", post(tool::invoke))
        .route("/api/tools/{slug}/env", get(tool::list_env).post(tool::set_env))
        .route("/api/tools/{slug}/env/{key}", delete(tool::delete_env))
        .route("/api/stats/summary", get(stats::summary))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state)
}

pub async fn start_server(
    addr: std::net::SocketAddr,
    state: std::sync::Arc<crate::AppState>,
) -> crate::Result<()> {
    let grpc_addr: std::net::SocketAddr = format!("0.0.0.0:{}", addr.port() + 1).parse().unwrap();
    
    let grpc_service = crate::grpc::cerebrate::swarm_service_server::SwarmServiceServer::new(
        crate::grpc::SwarmGrpcServer::new(state.clone())
    );
    
    let http_router = create_router(state);
    
    tracing::info!("HTTP REST API listening on {}", addr);
    tracing::info!("gRPC API listening on {}", grpc_addr);
    
    let http_task = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, http_router).await.unwrap();
    });
    
    tonic::transport::Server::builder()
        .add_service(grpc_service)
        .serve(grpc_addr)
        .await
        .map_err(|e| crate::Error::Internal(e.to_string()))?;

    http_task.abort();

    Ok(())
}