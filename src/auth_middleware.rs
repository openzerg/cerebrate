use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use tonic::{Request as TonicRequest, Status as TonicStatus};

pub fn extract_token_from_header(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

pub fn extract_token_from_grpc<T>(request: &TonicRequest<T>) -> Option<String> {
    request
        .metadata()
        .get("authorization")
        .and_then(|m| m.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

pub async fn auth_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token_from_header(request.headers());
    if token.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    let token = token.unwrap();
    match crate::jwt::decode_token(&token) {
        Ok(_claims) => Ok(next.run(request).await),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub fn get_claims_from_extensions<T>(request: &TonicRequest<T>) -> Option<&crate::jwt::Claims> {
    request.extensions().get::<crate::jwt::Claims>()
}

pub fn require_auth<T>(request: &TonicRequest<T>) -> Result<&crate::jwt::Claims, TonicStatus> {
    get_claims_from_extensions(request)
        .ok_or_else(|| TonicStatus::unauthenticated("Missing authentication"))
}