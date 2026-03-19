use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(String),

    #[error("Invalid token")]
    InvalidToken,

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Task failed: {0}")]
    TaskFailed(String),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_io() {
        let err = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_display_serialization() {
        let json = serde_json::from_str::<i32>("not a number");
        let err = Error::Serialization(json.unwrap_err());
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_display_not_found() {
        let err = Error::NotFound("resource".to_string());
        assert_eq!(err.to_string(), "Not found: resource");
    }

    #[test]
    fn test_error_display_validation() {
        let err = Error::Validation("invalid input".to_string());
        assert_eq!(err.to_string(), "Validation error: invalid input");
    }

    #[test]
    fn test_error_display_agent_not_found() {
        let err = Error::AgentNotFound("agent-1".to_string());
        assert_eq!(err.to_string(), "Agent not found: agent-1");
    }

    #[test]
    fn test_error_display_agent_already_exists() {
        let err = Error::AgentAlreadyExists("agent-1".to_string());
        assert_eq!(err.to_string(), "Agent already exists: agent-1");
    }

    #[test]
    fn test_error_display_invalid_token() {
        let err = Error::InvalidToken;
        assert_eq!(err.to_string(), "Invalid token");
    }

    #[test]
    fn test_error_display_websocket() {
        let err = Error::WebSocket("connection failed".to_string());
        assert_eq!(err.to_string(), "WebSocket error: connection failed");
    }

    #[test]
    fn test_error_display_task_failed() {
        let err = Error::TaskFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Task failed: timeout");
    }

    #[test]
    fn test_error_display_config() {
        let err = Error::Config("invalid".to_string());
        assert_eq!(err.to_string(), "Config error: invalid");
    }

    #[test]
    fn test_error_display_database() {
        let err = Error::Database("connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Serialization(_)));
    }

    #[test]
    fn test_from_serde_yaml_error() {
        let yaml_err = serde_yaml::from_str::<i32>("invalid: yaml: :");
        let err: Error = yaml_err.unwrap_err().into();
        assert!(matches!(err, Error::Yaml(_)));
    }

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(Error::NotFound("test".to_string()));
        assert!(result.is_err());
    }
}
