use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures_util::stream::{self, Stream};
use std::sync::Arc;
use std::convert::Infallible;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use reqwest::Client;

use crate::protocol::{AgentEventMessage, HostEvent};
use crate::AppState;
use super::types::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub question: String,
}

#[derive(Debug, Deserialize)]
pub struct SendEventRequest {
    #[serde(flatten)]
    pub event: AgentEventMessage,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct SseEvent {
    pub event_type: &'static str,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FileResponse {
    List(Vec<FileInfo>),
    Content(FileContent),
}

async fn get_agent_ip(state: &Arc<AppState>, name: &str) -> Option<String> {
    let connections = state.vm_connections.read().await;
    if connections.contains_key(name) {
        drop(connections);
        let state_data = state.state_manager.load().await.ok()?;
        state_data.agents.get(name).map(|a| a.container_ip.clone())
    } else {
        None
    }
}

pub async fn query_agent(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> impl IntoResponse {
    let query_id = uuid::Uuid::new_v4().to_string();
    
    let connections = state.vm_connections.read().await;
    if !connections.contains_key(&name) {
        return Err(StatusCode::NOT_FOUND);
    }
    drop(connections);

    let (tx, mut rx) = mpsc::channel::<serde_json::Value>(100);

    {
        let mut pending = state.pending_queries.write().await;
        pending.insert(query_id.clone(), tx);
    }

    let event_id = uuid::Uuid::new_v4().to_string();
    let event = HostEvent {
        event_id: event_id.clone(),
        event: AgentEventMessage::Query {
            query_id: query_id.clone(),
            question: req.question,
        },
    };

    let _ = state.event_tx.send(crate::protocol::AgentEvent {
        event: crate::protocol::AgentEventType::StatusUpdate,
        agent_name: name.clone(),
        timestamp: chrono::Utc::now(),
        data: Some(serde_json::to_value(&event).unwrap_or(serde_json::Value::Null)),
    });

    let stream = async_stream::stream! {
        yield Ok::<_, Infallible>(Event::default()
            .event("query_started")
            .json_data(&serde_json::json!({"query_id": query_id}))
            .unwrap());

        while let Some(event) = rx.recv().await {
            if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
                yield Ok::<_, Infallible>(Event::default()
                    .event(event_type)
                    .json_data(&event)
                    .unwrap());
            }
        }

        yield Ok::<_, Infallible>(Event::default().event("done").data("[done]"));
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn send_event(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendEventRequest>,
) -> impl IntoResponse {
    let connections = state.vm_connections.read().await;
    if !connections.contains_key(&name) {
        return Err(StatusCode::NOT_FOUND);
    }
    drop(connections);

    let event_id = uuid::Uuid::new_v4().to_string();
    let event = HostEvent {
        event_id: event_id.clone(),
        event: req.event,
    };

    let _ = state.event_tx.send(crate::protocol::AgentEvent {
        event: crate::protocol::AgentEventType::StatusUpdate,
        agent_name: name.clone(),
        timestamp: chrono::Utc::now(),
        data: Some(serde_json::to_value(&event).unwrap_or(serde_json::Value::Null)),
    });

    Ok(Json(ApiResponse {
        success: true,
        data: Some(event_id),
        error: None,
    }))
}

pub async fn list_files(
    Path(name): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let workspace = std::path::PathBuf::from("/var/lib/agents").join(&name);
    
    if !workspace.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&workspace) {
        for entry in entries.flatten() {
            let path = entry.path();
            let metadata = entry.metadata().ok();
            
            files.push(FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.strip_prefix(&workspace)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                is_dir: path.is_dir(),
                size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: metadata.as_ref().and_then(|m| {
                    m.modified().ok().map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
                    })
                }),
            });
        }
    }

    files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));

    Ok(Json(ApiResponse {
        success: true,
        data: Some(files),
        error: None,
    }))
}

pub async fn get_file(
    Path((name, path)): Path<(String, String)>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let workspace = std::path::PathBuf::from("/var/lib/agents").join(&name);
    let file_path = workspace.join(&path);
    
    if !file_path.exists() || !file_path.starts_with(&workspace) {
        return Err(StatusCode::NOT_FOUND);
    }

    if file_path.is_dir() {
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&file_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let metadata = entry.metadata().ok();
                
                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: p.strip_prefix(&workspace)
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    is_dir: p.is_dir(),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: metadata.as_ref().and_then(|m| {
                        m.modified().ok().map(|t| {
                            chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
                        })
                    }),
                });
            }
        }

        files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));

        return Ok(Json(ApiResponse::<FileResponse> {
            success: true,
            data: Some(FileResponse::List(files)),
            error: None,
        }));
    }

    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let size = content.len() as u64;

    Ok(Json(ApiResponse::<FileResponse> {
        success: true,
        data: Some(FileResponse::Content(FileContent {
            path: path.clone(),
            content,
            size,
        })),
        error: None,
    }))
}

pub async fn update_file(
    Path((name, path)): Path<(String, String)>,
    State(_state): State<Arc<AppState>>,
    Json(req): Json<UpdateFileRequest>,
) -> impl IntoResponse {
    let workspace = std::path::PathBuf::from("/var/lib/agents").join(&name);
    let file_path = workspace.join(&path);
    
    if !file_path.starts_with(&workspace) {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(parent) = file_path.parent() {
        if let Err(_) = std::fs::create_dir_all(parent) {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    if let Err(_) = std::fs::write(&file_path, &req.content) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ApiResponse {
        success: true,
        data: Some(path),
        error: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    pub content: String,
}

async fn proxy_request(
    state: Arc<AppState>,
    agent_name: &str,
    api_path: &str,
    method: reqwest::Method,
    query: Option<std::collections::HashMap<String, String>>,
    body: Option<serde_json::Value>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let ip = get_agent_ip(&state, agent_name).await
        .ok_or((StatusCode::NOT_FOUND, "Agent not found or offline".to_string()))?;
    
    let client = Client::new();
    let url = format!("http://{}:8081{}", ip, api_path);
    
    let mut req = match method {
        reqwest::Method::GET => client.get(&url),
        reqwest::Method::POST => client.post(&url),
        reqwest::Method::PUT => client.put(&url),
        reqwest::Method::DELETE => client.delete(&url),
        _ => return Err((StatusCode::METHOD_NOT_ALLOWED, "Method not allowed".to_string())),
    };
    
    if let Some(q) = query {
        req = req.query(&q);
    }
    
    if let Some(b) = body {
        req = req.json(&b);
    }
    
    match req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = resp.bytes().await.unwrap_or_default();
            Ok(axum::response::Response::builder()
                .status(status)
                .body(axum::body::Body::from(body))
                .unwrap())
        }
        Err(e) => {
            tracing::error!("Proxy error: {}", e);
            Err((StatusCode::BAD_GATEWAY, format!("Proxy error: {}", e)))
        }
    }
}

fn error_response(e: (StatusCode, String)) -> axum::response::Response {
    let (status, msg) = e;
    axum::response::Response::builder()
        .status(status)
        .body(axum::body::Body::from(serde_json::to_string(&ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(msg),
        }).unwrap_or_default()))
        .unwrap()
}

pub async fn proxy_sessions(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/sessions", reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_session(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/sessions/{}", id), reqwest::Method::GET, None, None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_session_messages(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/sessions/{}/messages", id), reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_session_chat(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/sessions/{}/chat", id), reqwest::Method::POST, None, Some(body)).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_session_interrupt(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/sessions/{}/interrupt", id), reqwest::Method::POST, None, Some(body)).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_processes(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/processes", reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_process(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/processes/{}", id), reqwest::Method::GET, None, None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_process_output(
    Path((name, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, &format!("/api/processes/{}/output", id), reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_tasks(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/tasks", reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_activities(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/activities", reqwest::Method::GET, Some(query), None).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_message(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/message", reqwest::Method::POST, None, Some(body)).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}

pub async fn proxy_remind(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    match proxy_request(state, &name, "/api/remind", reqwest::Method::POST, None, Some(body)).await {
        Ok(resp) => resp,
        Err(e) => error_response(e),
    }
}