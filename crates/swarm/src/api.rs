use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    routing::{get, post, delete},
    Json, Router, extract::{Path, State, ws::WebSocketUpgrade},
    middleware,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::{AppState, VmConnection};
use crate::forgejo;
use crate::auth::{AuthConfig, auth_middleware};
use crate::models::{CreateProviderRequest, CreateApiKeyRequest};
use crate::protocol::{Message, VmConnect, VmHeartbeat, AgentEvent, AgentEventType};

const DEFAULT_PORT: u16 = 17531;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub forgejo_username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateForgejoUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    pub agent: String,
    pub forgejo_user: String,
}

#[derive(Debug, Deserialize)]
pub struct UnbindRequest {
    pub agent: String,
}

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub enabled: bool,
    pub container_ip: String,
    pub host_ip: String,
    pub forgejo_username: Option<String>,
    pub online: bool,
}

#[derive(Debug, Serialize)]
pub struct ForgejoUserInfo {
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    pub exported_at: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_agents: usize,
    pub online_agents: usize,
    pub enabled_agents: usize,
}

pub async fn start_server(
    addr: SocketAddr, 
    state: Arc<AppState>,
    auth_config: AuthConfig,
) -> crate::Result<()> {
    let auth_state = Arc::new(auth_config);
    
    let api_routes = Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/{name}", get(get_agent).delete(delete_agent))
        .route("/agents/{name}/enable", post(enable_agent))
        .route("/agents/{name}/disable", post(disable_agent))
        .route("/agents/{name}/stats", get(get_agent_stats))
        .route("/stats/summary", get(get_stats_summary))
        .route("/apply", post(apply_config))
        .route("/git/users", get(list_forgejo_users).post(create_forgejo_user))
        .route("/git/users/{username}", delete(delete_forgejo_user))
        .route("/git/users/bind", post(bind_forgejo_user))
        .route("/git/users/unbind", post(unbind_forgejo_user))
        .route("/git/repos", get(list_git_repos).post(create_git_repo))
        .route("/git/repos/{owner}/{repo}", get(get_git_repo).delete(delete_git_repo).patch(update_git_repo))
        .route("/git/repos/{owner}/{repo}/transfer", post(transfer_git_repo))
        .route("/git/repos/{owner}/{repo}/collaborators", get(list_collaborators).post(add_collaborator))
        .route("/git/repos/{owner}/{repo}/collaborators/{username}", delete(remove_collaborator))
        .route("/git/orgs", get(list_orgs).post(create_org))
        .route("/git/orgs/{org}", delete(delete_org))
        .route("/git/orgs/{org}/members", get(list_org_members))
        .route("/git/orgs/{org}/members/{username}", post(add_org_member).delete(remove_org_member))
        .route("/config/export", get(export_config))
        .route("/config/import", post(import_config))
        .route("/llm/providers", get(list_providers).post(create_provider))
        .route("/llm/providers/{id}", delete(delete_provider))
        .route("/llm/providers/{id}/enable", post(enable_provider))
        .route("/llm/providers/{id}/disable", post(disable_provider))
        .route("/llm/keys", get(list_api_keys).post(create_api_key))
        .route("/llm/keys/{id}", delete(delete_api_key))
        .route_layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/ws", get(event_ws_handler))
        .route("/ws/vm", get(vm_ws_handler))
        .route("/v1/chat/completions", post(llm_chat_completion))
        .nest("/api", api_routes)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Host-worker listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn event_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_event_ws(socket, state))
}

async fn handle_event_ws(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();
    let mut event_rx = state.event_tx.subscribe();

    tracing::info!("Event subscriber connected");

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Ok(agent_event) => {
                        let msg = Message::AgentEvent(agent_event);
                        match msg.to_json() {
                            Ok(json) => {
                                if tx.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            msg = rx.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        if tx.send(axum::extract::ws::Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::info!("Event subscriber disconnected");
}

async fn vm_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_vm_ws(socket, state))
}

async fn handle_vm_ws(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();
    let mut agent_name: Option<String> = None;

    tracing::info!("VM WebSocket connection");

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                match Message::from_json(&text) {
                    Ok(message) => {
                        match message {
                            Message::VmConnect(connect) => {
                                match handle_vm_connect(&state, &connect).await {
                                    Ok(name) => agent_name = Some(name),
                                    Err(e) => {
                                        tracing::error!("VM connect error: {}", e);
                                        let _ = tx.send(axum::extract::ws::Message::Close(None)).await;
                                        break;
                                    }
                                }
                            }
                            Message::VmHeartbeat(heartbeat) => {
                                handle_vm_heartbeat(&state, &heartbeat).await;
                            }
                            Message::VmStatusReport(report) => {
                                let _ = state.event_tx.send(AgentEvent {
                                    event: AgentEventType::StatusUpdate,
                                    agent_name: report.agent_name.clone(),
                                    timestamp: chrono::Utc::now(),
                                    data: Some(serde_json::to_value(&report.data).unwrap_or(serde_json::Value::Null)),
                                });
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse message: {}", e);
                    }
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => break,
            Ok(axum::extract::ws::Message::Ping(data)) => {
                if tx.send(axum::extract::ws::Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    if let Some(name) = agent_name {
        let mut connections = state.vm_connections.write().await;
        if let Some(conn) = connections.get_mut(&name) {
            conn.connected = false;
        }
        drop(connections);
        
        let _ = state.event_tx.send(AgentEvent {
            event: AgentEventType::Disconnected,
            agent_name: name.clone(),
            timestamp: chrono::Utc::now(),
            data: None,
        });
        tracing::info!("Agent {} disconnected", name);
    }
}

async fn handle_vm_connect(state: &AppState, connect: &VmConnect) -> crate::Result<String> {
    let agent = state.db.get_agent(&connect.agent_name).await?
        .ok_or_else(|| crate::Error::AgentNotFound(connect.agent_name.clone()))?;

    if agent.internal_token != connect.internal_token {
        return Err(crate::Error::InvalidToken);
    }

    {
        let mut connections = state.vm_connections.write().await;
        connections.insert(connect.agent_name.clone(), VmConnection {
            agent_name: connect.agent_name.clone(),
            connected: true,
            last_heartbeat: chrono::Utc::now(),
        });
    }

    let _ = state.event_tx.send(AgentEvent {
        event: AgentEventType::Connected,
        agent_name: connect.agent_name.clone(),
        timestamp: chrono::Utc::now(),
        data: None,
    });

    tracing::info!("Agent {} connected", connect.agent_name);
    Ok(connect.agent_name.clone())
}

async fn handle_vm_heartbeat(state: &AppState, heartbeat: &VmHeartbeat) {
    let mut connections = state.vm_connections.write().await;
    if let Some(conn) = connections.get_mut(&heartbeat.agent_name) {
        conn.last_heartbeat = chrono::Utc::now();
        conn.connected = true;
    }
}

async fn llm_chat_completion(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<super::proxy::ChatCompletionRequest>,
) -> Result<Json<super::proxy::ChatCompletionResponse>, super::proxy::ProxyError> {
    use axum::http::header;
    
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(super::proxy::ProxyError::MissingAuth)?;

    let api_key = auth
        .strip_prefix("Bearer ")
        .ok_or(super::proxy::ProxyError::InvalidAuthFormat)?;

    let key_hash = crate::db::Database::hash_key(api_key);
    let (_, provider) = state.db.get_api_key_by_hash(&key_hash).await
        .map_err(|e| super::proxy::ProxyError::UpstreamError(e.to_string()))?
        .ok_or(super::proxy::ProxyError::InvalidApiKey)?;

    if !provider.enabled {
        return Err(super::proxy::ProxyError::ProviderDisabled(provider.name));
    }

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", provider.base_url.trim_end_matches('/'));
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| super::proxy::ProxyError::UpstreamError(e.to_string()))?;

    let completion = response.json::<super::proxy::ChatCompletionResponse>().await
        .map_err(|e| super::proxy::ProxyError::UpstreamError(e.to_string()))?;

    Ok(Json(completion))
}

async fn get_agent_stats(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match state.db.get_agent(&name).await {
        Ok(Some(_)) => {
            let connections = state.vm_connections.read().await;
            let conn = connections.get(&name);
            let stats = serde_json::json!({
                "online": conn.map(|c| c.connected).unwrap_or(false),
                "last_heartbeat": conn.map(|c| c.last_heartbeat.to_rfc3339()).unwrap_or_default(),
            });
            Json(ApiResponse::success(stats))
        }
        Ok(None) => Json(ApiResponse::error("Agent not found")),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn get_stats_summary(State(state): State<Arc<AppState>>) -> Json<ApiResponse<StatsSummary>> {
    match state.db.list_agents().await {
        Ok(agents) => {
            let connections = state.vm_connections.read().await;
            let online = agents.iter().filter(|a| {
                connections.get(&a.name).map(|c| c.connected).unwrap_or(false)
            }).count();
            let enabled = agents.iter().filter(|a| a.enabled).count();
            Json(ApiResponse::success(StatsSummary {
                total_agents: agents.len(),
                online_agents: online,
                enabled_agents: enabled,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn list_agents(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<AgentInfo>>> {
    match state.db.list_agents().await {
        Ok(agents) => {
            let connections = state.vm_connections.read().await;
            let infos: Vec<AgentInfo> = agents.into_iter().map(|a| {
                let online = connections.get(&a.name).map(|c| c.connected).unwrap_or(false);
                AgentInfo {
                    name: a.name, enabled: a.enabled,
                    container_ip: a.container_ip, host_ip: a.host_ip,
                    forgejo_username: a.forgejo_username, online,
                }
            }).collect();
            Json(ApiResponse::success(infos))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AgentInfo>> {
    match state.db.get_agent(&name).await {
        Ok(Some(agent)) => {
            let online = state.vm_connections.read().await
                .get(&name).map(|c| c.connected).unwrap_or(false);
            Json(ApiResponse::success(AgentInfo {
                name: agent.name.clone(), enabled: agent.enabled,
                container_ip: agent.container_ip.clone(), host_ip: agent.host_ip.clone(),
                forgejo_username: agent.forgejo_username.clone(), online,
            }))
        }
        Ok(None) => Json(ApiResponse::error("Agent not found")),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Json<ApiResponse<AgentInfo>> {
    match state.db.get_agent(&req.name).await {
        Ok(Some(_)) => return Json(ApiResponse::error("Agent already exists")),
        Ok(None) => {}
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    }

    let agent_num = match state.db.get_next_agent_num().await {
        Ok(n) => n,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    let now = chrono::Utc::now().to_rfc3339();
    
    let agent = crate::models::Agent {
        name: req.name.clone(),
        enabled: true,
        container_ip: format!("{}.{}.2", defaults.container_subnet_base, agent_num),
        host_ip: format!("{}.{}.1", defaults.container_subnet_base, agent_num),
        forgejo_username: req.forgejo_username.or(Some(req.name)),
        internal_token: uuid::Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    match state.db.create_agent(&agent).await {
        Ok(()) => Json(ApiResponse::success(AgentInfo {
            name: agent.name, enabled: agent.enabled,
            container_ip: agent.container_ip, host_ip: agent.host_ip,
            forgejo_username: agent.forgejo_username, online: false,
        })),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.get_agent(&name).await {
        Ok(Some(_)) => match state.db.delete_agent(&name).await {
            Ok(()) => Json(ApiResponse::success(())),
            Err(e) => Json(ApiResponse::error(e.to_string())),
        },
        Ok(None) => Json(ApiResponse::error("Agent not found")),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn enable_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.update_agent_enabled(&name, true).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn disable_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.update_agent_enabled(&name, false).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn apply_config(State(state): State<Arc<AppState>>) -> Json<ApiResponse<String>> {
    match state.db.list_agents().await {
        Ok(agents) => {
            let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
            match state.agent_manager.apply_config(&agents, &defaults).await {
                Ok(()) => Json(ApiResponse::success("NixOS configuration applied successfully".to_string())),
                Err(e) => Json(ApiResponse::error(e.to_string())),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn list_forgejo_users(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<ForgejoUserInfo>>> {
    match state.db.list_forgejo_users().await {
        Ok(users) => {
            let infos: Vec<ForgejoUserInfo> = users.into_iter().map(|u| ForgejoUserInfo {
                username: u.username,
                email: u.email,
                created_at: u.created_at,
            }).collect();
            Json(ApiResponse::success(infos))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn create_forgejo_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateForgejoUserRequest>,
) -> Json<ApiResponse<ForgejoUserInfo>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    
    match forgejo::create_user(&state.db, &defaults.forgejo_url, forgejo_token, &req.username, &req.password).await {
        Ok(()) => {
            match state.db.get_forgejo_user(&req.username).await {
                Ok(Some(user)) => Json(ApiResponse::success(ForgejoUserInfo {
                    username: user.username,
                    email: user.email,
                    created_at: user.created_at,
                })),
                _ => Json(ApiResponse::error("User created but not found in database")),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_forgejo_user(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Json<ApiResponse<()>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    
    match forgejo::delete_user(&state.db, &defaults.forgejo_url, forgejo_token, &username).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn bind_forgejo_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BindRequest>,
) -> Json<ApiResponse<()>> {
    match state.db.get_agent(&req.agent).await {
        Ok(None) => Json(ApiResponse::error("Agent not found")),
        Ok(Some(_)) => {
            match state.db.get_forgejo_user(&req.forgejo_user).await {
                Ok(None) => Json(ApiResponse::error("Forgejo user not found")),
                Ok(Some(_)) => {
                    match state.db.bind_forgejo_user(&req.agent, &req.forgejo_user).await {
                        Ok(()) => Json(ApiResponse::success(())),
                        Err(e) => Json(ApiResponse::error(e.to_string())),
                    }
                }
                Err(e) => Json(ApiResponse::error(e.to_string())),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn unbind_forgejo_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UnbindRequest>,
) -> Json<ApiResponse<()>> {
    match state.db.get_agent(&req.agent).await {
        Ok(None) => Json(ApiResponse::error("Agent not found")),
        Ok(Some(_)) => {
            match state.db.unbind_forgejo_user(&req.agent).await {
                Ok(()) => Json(ApiResponse::success(())),
                Err(e) => Json(ApiResponse::error(e.to_string())),
            }
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn export_config(State(state): State<Arc<AppState>>) -> Json<ApiResponse<ConfigInfo>> {
    let export_path = state.data_dir.join("config.yaml");
    
    match crate::config::export_to_yaml(&state.db, &export_path).await {
        Ok(()) => Json(ApiResponse::success(ConfigInfo {
            exported_at: chrono::Utc::now().to_rfc3339(),
            path: export_path.to_string_lossy().to_string(),
        })),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn import_config(State(state): State<Arc<AppState>>) -> Json<ApiResponse<String>> {
    let import_path = state.data_dir.join("config.yaml");
    
    match crate::config::import_from_yaml(&state.db, &import_path).await {
        Ok(()) => Json(ApiResponse::success(format!("Config imported from {:?}", import_path))),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn list_providers(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<ProviderInfo>>> {
    match state.db.list_providers().await {
        Ok(providers) => {
            let infos: Vec<ProviderInfo> = providers.into_iter().map(|p| ProviderInfo {
                id: p.id,
                name: p.name,
                provider_type: p.provider_type.as_str().to_string(),
                base_url: p.base_url,
                enabled: p.enabled,
                created_at: p.created_at,
            }).collect();
            Json(ApiResponse::success(infos))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> Json<ApiResponse<ProviderInfo>> {
    match state.db.create_provider(&req).await {
        Ok(p) => Json(ApiResponse::success(ProviderInfo {
            id: p.id,
            name: p.name,
            provider_type: p.provider_type.as_str().to_string(),
            base_url: p.base_url,
            enabled: p.enabled,
            created_at: p.created_at,
        })),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.delete_provider(&id).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn enable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.update_provider_enabled(&id, true).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn disable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.update_provider_enabled(&id, false).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub key: Option<String>,
    pub created_at: String,
}

async fn list_api_keys(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<ApiKeyInfo>>> {
    match state.db.list_api_keys().await {
        Ok(keys) => {
            let providers = state.db.list_providers().await.unwrap_or_default();
            let provider_map: std::collections::HashMap<_, _> = providers.into_iter().map(|p| (p.id, p.name)).collect();
            let infos: Vec<ApiKeyInfo> = keys.into_iter().map(|k| ApiKeyInfo {
                id: k.id,
                name: k.name,
                provider_id: k.provider_id.clone(),
                provider_name: provider_map.get(&k.provider_id).cloned().unwrap_or_default(),
                key: None,
                created_at: k.created_at,
            }).collect();
            Json(ApiResponse::success(infos))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Json<ApiResponse<ApiKeyInfo>> {
    match state.db.create_api_key(&req).await {
        Ok((k, raw_key)) => {
            let provider = state.db.get_provider(&k.provider_id).await.ok().flatten();
            Json(ApiResponse::success(ApiKeyInfo {
                id: k.id,
                name: k.name,
                provider_id: k.provider_id.clone(),
                provider_name: provider.map(|p| p.name).unwrap_or_default(),
                key: Some(raw_key),
                created_at: k.created_at,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    match state.db.delete_api_key(&id).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct RepoPath {
    owner: String,
    repo: String,
}

#[derive(Debug, Deserialize)]
struct CreateRepoRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransferRepoRequest {
    new_owner: String,
}

#[derive(Debug, Deserialize)]
struct UpdateRepoRequest {
    private: Option<bool>,
    description: Option<String>,
}

async fn list_git_repos(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<Vec<forgejo::Repository>>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    let owner = params.get("owner").map(|s| s.as_str());
    match forgejo::list_repos(&defaults.forgejo_url, forgejo_token, owner).await {
        Ok(repos) => Json(ApiResponse::success(repos)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn get_git_repo(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
) -> Json<ApiResponse<forgejo::Repository>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::get_repo(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo).await {
        Ok(Some(repo)) => Json(ApiResponse::success(repo)),
        Ok(None) => Json(ApiResponse::error("Repository not found")),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn create_git_repo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRepoRequest>,
) -> Json<ApiResponse<forgejo::Repository>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::create_repo(&defaults.forgejo_url, forgejo_token, &req.name, req.description.as_deref()).await {
        Ok(repo) => Json(ApiResponse::success(repo)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_git_repo(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
) -> Json<ApiResponse<()>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::delete_repo(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn transfer_git_repo(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
    Json(req): Json<TransferRepoRequest>,
) -> Json<ApiResponse<()>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::transfer_repo(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo, &req.new_owner).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn update_git_repo(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
    Json(req): Json<UpdateRepoRequest>,
) -> Json<ApiResponse<forgejo::Repository>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::update_repo(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo, req.private, req.description.as_deref()).await {
        Ok(repo) => Json(ApiResponse::success(repo)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct AddCollaboratorRequest {
    username: String,
    permission: Option<String>,
}

async fn list_collaborators(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
) -> Json<ApiResponse<Vec<forgejo::Collaborator>>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::list_collaborators(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo).await {
        Ok(collaborators) => Json(ApiResponse::success(collaborators)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn add_collaborator(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoPath>,
    Json(req): Json<AddCollaboratorRequest>,
) -> Json<ApiResponse<()>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::add_collaborator(&defaults.forgejo_url, forgejo_token, &path.owner, &path.repo, &req.username, req.permission.as_deref()).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn remove_collaborator(
    State(state): State<Arc<AppState>>,
    Path(path): Path<(String, String, String)>,
) -> Json<ApiResponse<()>> {
    let (owner, repo, username) = path;
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::remove_collaborator(&defaults.forgejo_url, forgejo_token, &owner, &repo, &username).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn list_orgs(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<forgejo::Organization>>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::list_orgs(&defaults.forgejo_url, forgejo_token).await {
        Ok(orgs) => Json(ApiResponse::success(orgs)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct CreateOrgRequest {
    name: String,
}

async fn create_org(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOrgRequest>,
) -> Json<ApiResponse<forgejo::Organization>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::create_org(&defaults.forgejo_url, forgejo_token, &req.name).await {
        Ok(org) => Json(ApiResponse::success(org)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn delete_org(
    State(state): State<Arc<AppState>>,
    Path(org): Path<String>,
) -> Json<ApiResponse<()>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::delete_org(&defaults.forgejo_url, forgejo_token, &org).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn list_org_members(
    State(state): State<Arc<AppState>>,
    Path(org): Path<String>,
) -> Json<ApiResponse<Vec<forgejo::OrgMember>>> {
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::list_org_members(&defaults.forgejo_url, forgejo_token, &org).await {
        Ok(members) => Json(ApiResponse::success(members)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn add_org_member(
    State(state): State<Arc<AppState>>,
    Path(path): Path<(String, String)>,
) -> Json<ApiResponse<()>> {
    let (org, username) = path;
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::add_org_member(&defaults.forgejo_url, forgejo_token, &org, &username).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

async fn remove_org_member(
    State(state): State<Arc<AppState>>,
    Path(path): Path<(String, String)>,
) -> Json<ApiResponse<()>> {
    let (org, username) = path;
    let defaults = state.db.get_defaults().await;
    let forgejo_token = &defaults.forgejo_token;
    match forgejo::remove_org_member(&defaults.forgejo_url, forgejo_token, &org, &username).await {
        Ok(()) => Json(ApiResponse::success(())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}