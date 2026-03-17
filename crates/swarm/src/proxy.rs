use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: ChatCompletionUsage,
}

#[derive(Debug)]
pub enum ProxyError {
    MissingAuth,
    InvalidAuthFormat,
    InvalidApiKey,
    ProviderDisabled(String),
    UpstreamError(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> axum::response::Response {
        let (status, message): (StatusCode, String) = match self {
            Self::MissingAuth => (
                StatusCode::UNAUTHORIZED,
                "Missing authorization header".to_string(),
            ),
            Self::InvalidAuthFormat => (
                StatusCode::UNAUTHORIZED,
                "Invalid authorization format".to_string(),
            ),
            Self::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key".to_string()),
            Self::ProviderDisabled(name) => (
                StatusCode::FORBIDDEN,
                format!("Provider {} is disabled", name),
            ),
            Self::UpstreamError(e) => (StatusCode::BAD_GATEWAY, e),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
