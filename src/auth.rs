use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::models::CallerIdentity;
use crate::state::StateManager;

pub struct AuthState {
    pub state_manager: Arc<StateManager>,
}

pub struct AuthenticatedCaller(pub CallerIdentity);

impl<S> FromRequestParts<S> for AuthenticatedCaller
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CallerIdentity>()
            .cloned()
            .map(AuthenticatedCaller)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn auth_middleware(
    axum::extract::State(auth_state): axum::extract::State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let header = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    if !header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &header[7..];

    let sw = auth_state
        .state_manager
        .load()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(ref admin_token) = sw.admin_token {
        if token == admin_token {
            request.extensions_mut().insert(CallerIdentity::Admin);
            return Ok(next.run(request).await);
        }
    }

    for (name, agent) in &sw.agents {
        if &agent.internal_token == token {
            request
                .extensions_mut()
                .insert(CallerIdentity::Agent(name.clone()));
            return Ok(next.run(request).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_creation() {
        let state_manager = Arc::new(StateManager::new(std::path::Path::new("/tmp/test")));
        let auth_state = AuthState { state_manager };
        assert!(Arc::strong_count(&auth_state.state_manager) >= 1);
    }

    #[test]
    fn test_authenticated_caller_type() {
        let caller = AuthenticatedCaller(CallerIdentity::Admin);
        match caller.0 {
            CallerIdentity::Admin => (),
            CallerIdentity::Agent(_) => panic!("Expected Admin"),
        }
    }

    #[test]
    fn test_authenticated_caller_agent() {
        let caller = AuthenticatedCaller(CallerIdentity::Agent("test-agent".to_string()));
        match caller.0 {
            CallerIdentity::Agent(name) => assert_eq!(name, "test-agent"),
            CallerIdentity::Admin => panic!("Expected Agent"),
        }
    }
}