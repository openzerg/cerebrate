use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::{AppState, Error};
use crate::jwt::{self, Claims};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub jwt: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub subject: String,
    pub role: String,
}

pub async fn login(
    State(state): State<std::sync::Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Json<crate::api::ApiResponse<LoginResponse>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(crate::api::ApiResponse::err(&e.to_string())),
    };
    
    let admin_token = match sw.admin_token.as_ref() {
        Some(t) => t,
        None => return Json(crate::api::ApiResponse::err("Admin token not configured")),
    };
    
    if req.token != *admin_token {
        return Json(crate::api::ApiResponse::err("Invalid token"));
    }
    
    let claims = Claims::new_admin("admin");
    let jwt = match jwt::encode_token(&claims) {
        Ok(t) => t,
        Err(e) => return Json(crate::api::ApiResponse::err(&format!("Failed to generate JWT: {}", e))),
    };
    
    Json(crate::api::ApiResponse::ok(LoginResponse {
        jwt,
        expires_in: 24 * 3600,
    }))
}

pub async fn verify(
    Json(jwt_str): Json<String>,
) -> Json<crate::api::ApiResponse<VerifyResponse>> {
    match jwt::decode_token(&jwt_str) {
        Ok(claims) => Json(crate::api::ApiResponse::ok(VerifyResponse {
            valid: true,
            subject: claims.sub,
            role: claims.role,
        })),
        Err(_) => Json(crate::api::ApiResponse::ok(VerifyResponse {
            valid: false,
            subject: String::new(),
            role: String::new(),
        })),
    }
}