use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    pub const AGENT_NOT_FOUND: i32 = -32001;
    pub const UNAUTHORIZED: i32 = -32002;
    pub const NOT_FOUND: i32 = -32003;
    pub const ALREADY_EXISTS: i32 = -32004;

    pub fn parse_error() -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: "Parse error".into(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: "Invalid Request".into(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: "Method not found".into(),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: msg.into(),
            data: None,
        }
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: msg.into(),
            data: None,
        }
    }

    pub fn agent_not_found(name: &str) -> Self {
        Self {
            code: Self::AGENT_NOT_FOUND,
            message: format!("Agent '{}' not found or offline", name),
            data: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            code: Self::UNAUTHORIZED,
            message: "Unauthorized".into(),
            data: None,
        }
    }

    pub fn not_found(what: &str) -> Self {
        Self {
            code: Self::NOT_FOUND,
            message: format!("{} not found", what),
            data: None,
        }
    }

    pub fn already_exists(what: &str) -> Self {
        Self {
            code: Self::ALREADY_EXISTS,
            message: format!("{} already exists", what),
            data: None,
        }
    }
}

impl RpcResponse {
    pub fn success(id: Option<i64>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<i64>, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl RpcRequest {
    pub fn parse(json: &str) -> Result<Self, RpcError> {
        serde_json::from_str(json).map_err(|_| RpcError::parse_error())
    }

    pub fn to_json(&self) -> Result<String, RpcError> {
        serde_json::to_string(self).map_err(|_| RpcError::internal_error("Serialization failed"))
    }
}

impl RpcResponse {
    pub fn parse(json: &str) -> Result<Self, RpcError> {
        serde_json::from_str(json).map_err(|_| RpcError::parse_error())
    }

    pub fn to_json(&self) -> Result<String, RpcError> {
        serde_json::to_string(self).map_err(|_| RpcError::internal_error("Serialization failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(1),
            method: "agent.list".into(),
            params: None,
        };
        let json = req.to_json().unwrap();
        assert!(json.contains("agent.list"));
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"agent.list"}"#;
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "agent.list");
        assert_eq!(req.id, Some(1));
    }

    #[test]
    fn test_response_success() {
        let resp = RpcResponse::success(Some(1), serde_json::json!({"name": "test"}));
        let json = resp.to_json().unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error() {
        let resp = RpcResponse::error(Some(1), RpcError::method_not_found());
        let json = resp.to_json().unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(RpcError::PARSE_ERROR, -32700);
        assert_eq!(RpcError::METHOD_NOT_FOUND, -32601);
        assert_eq!(RpcError::AGENT_NOT_FOUND, -32001);
    }
}
