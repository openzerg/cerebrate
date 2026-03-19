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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_serde() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello");
    }

    #[test]
    fn test_chat_completion_request_serde() {
        let req = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("temperature"));
        assert!(json.contains("max_tokens"));
        assert!(json.contains("stream"));
    }

    #[test]
    fn test_chat_completion_request_minimal() {
        let req = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn test_chat_completion_response_serde() {
        let resp = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello!".to_string(),
                },
                finish_reason: "stop".to_string(),
            }],
            usage: ChatCompletionUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ChatCompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "chatcmpl-123");
        assert_eq!(parsed.choices.len(), 1);
        assert_eq!(parsed.usage.total_tokens, 15);
    }

    #[test]
    fn test_chat_completion_choice_serde() {
        let choice = ChatCompletionChoice {
            index: 1,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "Response".to_string(),
            },
            finish_reason: "length".to_string(),
        };
        let json = serde_json::to_string(&choice).unwrap();
        let parsed: ChatCompletionChoice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.index, 1);
        assert_eq!(parsed.finish_reason, "length");
    }

    #[test]
    fn test_chat_completion_usage_serde() {
        let usage = ChatCompletionUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let parsed: ChatCompletionUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_tokens, 150);
    }

    #[test]
    fn test_proxy_error_debug() {
        let err = ProxyError::MissingAuth;
        assert!(format!("{:?}", err).contains("MissingAuth"));
    }
}
