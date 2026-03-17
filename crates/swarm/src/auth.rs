use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub struct AuthConfig {
    pub username: String,
    pub password: String,
}

pub async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<std::sync::Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        if header.starts_with("Basic ") {
            let encoded = &header[6..];
            if let Ok(decoded) = BASE64.decode(encoded) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let (username, password) = (parts[0], parts[1]);
                        if username == config.username 
                            && password == config.password {
                            return Ok(next.run(request).await);
                        }
                    }
                }
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}