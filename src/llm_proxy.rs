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
use serde::{Deserialize, Serialize};

const LLM_PROXY_PORT: u16 = 17534;

#[derive(Debug, Serialize)]
struct ProxyError {
    error: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    #[serde(default)]
    messages: Vec<serde_json::Value>,
}

pub async fn start_llm_proxy(state: Arc<AppState>) -> crate::Result<()> {
    let addr: SocketAddr = format!("0.0.0.0:{}", LLM_PROXY_PORT).parse().unwrap();
    
    let app = Router::new()
        .route("/v1/chat/completions", post(proxy_chat_completions))
        .route("/v1/models", get(proxy_models))
        .route("/v1/*path", post(proxy_generic))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("LLM Proxy listening on {}", addr);
    
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

async fn find_agent_by_token(state: &AppState, token: &str) -> Option<(String, crate::models::Agent)> {
    let sw = state.state_manager.load().await.ok()?;
    
    for (name, agent) in sw.agents.iter() {
        if agent.internal_token == token {
            return Some((name.clone(), agent.clone()));
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
    
    let (_agent_name, agent) = find_agent_by_token(&state, &token).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let model_id = agent.model_id.as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let sw = state.state_manager.load().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let model = sw.models.get(model_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    if !model.enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let provider = sw.providers.get(&model.provider_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    if !provider.enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let mut req_body: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    req_body["model"] = serde_json::json!(model.model_name);
    
    let target_url = format!("{}/v1/chat/completions", provider.base_url);
    
    forward_request(&target_url, &provider.api_key, &req_body).await
}

async fn proxy_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response<Body>, StatusCode> {
    let token = extract_token(&headers).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let (_agent_name, agent) = find_agent_by_token(&state, &token).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let model_id = agent.model_id.as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let sw = state.state_manager.load().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let model = sw.models.get(model_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let provider = sw.providers.get(&model.provider_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let target_url = format!("{}/v1/models", provider.base_url);
    
    let client = reqwest::Client::new();
    let resp = client
        .get(&target_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let status = resp.status();
    let body = resp.text().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK);
    
    Ok(response)
}

async fn proxy_generic(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response<Body>, StatusCode> {
    let token = extract_token(&headers).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let (_agent_name, agent) = find_agent_by_token(&state, &token).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let model_id = agent.model_id.as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let sw = state.state_manager.load().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let model = sw.models.get(model_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let provider = sw.providers.get(&model.provider_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let target_url = format!("{}/v1", provider.base_url);
    
    let client = reqwest::Client::new();
    let resp = client
        .post(&target_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let status = resp.status();
    let body = resp.text().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK);
    
    Ok(response)
}

async fn forward_request(target_url: &str, api_key: &str, body: &serde_json::Value) -> Result<Response<Body>, StatusCode> {
    let client = reqwest::Client::new();
    
    let resp = client
        .post(target_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let status = resp.status();
    let body = resp.text().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK);
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_proxy_port() {
        assert_eq!(LLM_PROXY_PORT, 17534);
    }
}