use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post},
    extract::State,
    http::{HeaderMap, StatusCode},
    body::Body,
    response::Response,
};
use crate::AppState;
use crate::pylon_client::PylonError;
use serde::Deserialize;

const LLM_PROXY_PORT: u16 = 17534;

#[derive(Debug, Deserialize)]
struct ChatCompletionRequest {
    model: String,
}

pub async fn start_llm_proxy(state: Arc<AppState>) -> crate::Result<()> {
    let addr: SocketAddr = format!("0.0.0.0:{}", LLM_PROXY_PORT).parse().unwrap();
    
    let app = Router::new()
        .route("/chat/completions", post(proxy_chat_completions))
        .route("/models", get(proxy_models))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("LLM Proxy (Pylon gateway) listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn extract_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?.to_str().ok()?;
    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

async fn find_agent_by_token(state: &AppState, token: &str) -> Option<String> {
    let sw = state.state_manager.load().await.ok()?;
    
    for (name, agent) in sw.agents.iter() {
        if agent.internal_token == token {
            return Some(name.clone());
        }
    }
    
    None
}

async fn proxy_chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response<Body>, StatusCode> {
    let token = extract_token(&headers).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let agent_name = find_agent_by_token(&state, &token).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let req_body: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let result = state.pylon_client.chat_completions(&agent_name, &req_body).await;
    
    match result {
        Ok(response) => {
            let body = serde_json::to_string(&response).unwrap_or_default();
            Ok(Response::new(Body::from(body)))
        }
        Err(PylonError::Unauthorized) => Err(StatusCode::UNAUTHORIZED),
        Err(PylonError::Forbidden) => Err(StatusCode::FORBIDDEN),
        Err(PylonError::NotFound(msg)) => {
            let error = serde_json::json!({"error": {"message": msg, "type": "not_found"}});
            let mut resp = Response::new(Body::from(error.to_string()));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
        Err(e) => {
            tracing::error!("Pylon error: {}", e);
            let error = serde_json::json!({"error": {"message": e.to_string(), "type": "internal_error"}});
            let mut resp = Response::new(Body::from(error.to_string()));
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(resp)
        }
    }
}

async fn proxy_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response<Body>, StatusCode> {
    let token = extract_token(&headers).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let agent_name = find_agent_by_token(&state, &token).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    match state.pylon_client.list_models().await {
        Ok(models) => {
            let data: Vec<serde_json::Value> = models.iter().map(|m| {
                serde_json::json!({
                    "id": m,
                    "object": "model",
                    "created": 1700000000,
                    "owned_by": "pylon"
                })
            }).collect();
            
            let response = serde_json::json!({
                "object": "list",
                "data": data
            });
            
            Ok(Response::new(Body::from(response.to_string())))
        }
        Err(e) => {
            tracing::error!("Failed to list models: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_proxy_port() {
        assert_eq!(LLM_PROXY_PORT, 17534);
    }
}