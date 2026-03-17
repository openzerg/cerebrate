use std::sync::Arc;
use axum::{
    Json, extract::{Path, State},
};
use crate::AppState;
use super::types::{ApiResponse, ProviderInfo};
use crate::models::{CreateProviderRequest, CreateApiKeyRequest, Provider, ApiKey};

pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<ProviderInfo>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let providers: Vec<ProviderInfo> = sw.providers.values().map(|p| ProviderInfo {
        id: p.id.clone(),
        name: p.name.clone(),
        provider_type: p.provider_type.as_str().to_string(),
        base_url: p.base_url.clone(),
        enabled: p.enabled,
        created_at: p.created_at.clone(),
    }).collect();
    
    Json(ApiResponse::success(providers))
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> Json<ApiResponse<ProviderInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let provider = Provider {
        id: id.clone(),
        name: req.name.clone(),
        provider_type: req.provider_type.clone(),
        base_url: req.base_url.clone(),
        api_key: req.api_key.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.providers.insert(id.clone(), provider.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(ProviderInfo {
        id,
        name: req.name,
        provider_type: req.provider_type.as_str().to_string(),
        base_url: req.base_url,
        enabled: true,
        created_at: provider.created_at,
    }))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.providers.remove(&id).is_none() {
        return Json(ApiResponse::error(format!("Provider '{}' not found", id)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Provider '{}' deleted", id)))
}

pub async fn enable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.providers.get_mut(&id) {
        Some(p) => {
            p.enabled = true;
            p.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Provider '{}' not found", id))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Provider '{}' enabled", id)))
}

pub async fn disable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.providers.get_mut(&id) {
        Some(p) => {
            p.enabled = false;
            p.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Provider '{}' not found", id))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Provider '{}' disabled", id)))
}

pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<crate::models::ApiKey>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let keys: Vec<ApiKey> = sw.api_keys.values().cloned().collect();
    Json(ApiResponse::success(keys))
}

pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let id = uuid::Uuid::new_v4().to_string();
    let raw_key = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let key_hash = format!("{:x}", hasher.finalize());
    
    let api_key = ApiKey {
        id: id.clone(),
        name: req.name.clone(),
        key_hash,
        provider_id: req.provider_id.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.api_keys.insert(id, api_key);
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(raw_key))
}

pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.api_keys.remove(&id).is_none() {
        return Json(ApiResponse::error(format!("API key '{}' not found", id)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("API key '{}' deleted", id)))
}