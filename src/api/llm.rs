use axum::{extract::{State, Path}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use super::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model_name: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub provider_id: String,
    pub model_name: String,
}

pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Provider>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let providers: Vec<Provider> = sw.providers.iter().map(|(id, p)| Provider {
        id: id.clone(),
        name: p.name.clone(),
        provider_type: p.provider_type.as_str().to_string(),
        base_url: p.base_url.clone(),
        enabled: p.enabled,
        created_at: p.created_at.clone(),
    }).collect();
    
    Json(ApiResponse::ok(providers))
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> Json<ApiResponse<Provider>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let provider_type = match crate::models::ProviderType::from_str(&req.provider_type) {
        Some(pt) => pt,
        None => return Json(ApiResponse::err(&format!("Invalid provider type: {}", req.provider_type))),
    };
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let provider = crate::models::Provider {
        id: id.clone(),
        name: req.name.clone(),
        provider_type,
        base_url: req.base_url.clone(),
        api_key: req.api_key.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.providers.insert(id.clone(), provider.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(Provider {
        id,
        name: provider.name,
        provider_type: provider.provider_type.as_str().to_string(),
        base_url: provider.base_url,
        enabled: provider.enabled,
        created_at: provider.created_at,
    }))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if sw.providers.remove(&id).is_none() {
        return Json(ApiResponse::err(&format!("Provider {} not found", id)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn enable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let provider = match sw.providers.get_mut(&id) {
        Some(p) => p,
        None => return Json(ApiResponse::err(&format!("Provider {} not found", id))),
    };
    
    provider.enabled = true;
    provider.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn disable_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let provider = match sw.providers.get_mut(&id) {
        Some(p) => p,
        None => return Json(ApiResponse::err(&format!("Provider {} not found", id))),
    };
    
    provider.enabled = false;
    provider.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<Model>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let models: Vec<Model> = sw.models.iter().map(|(id, m)| {
        let provider_name = sw.providers.get(&m.provider_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        
        Model {
            id: id.clone(),
            name: m.name.clone(),
            provider_id: m.provider_id.clone(),
            provider_name,
            model_name: m.model_name.clone(),
            enabled: m.enabled,
            created_at: m.created_at.clone(),
        }
    }).collect();
    
    Json(ApiResponse::ok(models))
}

pub async fn create_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateModelRequest>,
) -> Json<ApiResponse<Model>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if !sw.providers.contains_key(&req.provider_id) {
        return Json(ApiResponse::err(&format!("Provider {} not found", req.provider_id)));
    }
    
    let provider_name = sw.providers.get(&req.provider_id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let model = crate::models::Model {
        id: id.clone(),
        name: req.name.clone(),
        provider_id: req.provider_id.clone(),
        model_name: req.model_name.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };
    
    sw.models.insert(id.clone(), model.clone());
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(Model {
        id,
        name: model.name,
        provider_id: model.provider_id,
        provider_name,
        model_name: model.model_name,
        enabled: model.enabled,
        created_at: model.created_at,
    }))
}

pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    if sw.models.remove(&id).is_none() {
        return Json(ApiResponse::err(&format!("Model {} not found", id)));
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn enable_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let model = match sw.models.get_mut(&id) {
        Some(m) => m,
        None => return Json(ApiResponse::err(&format!("Model {} not found", id))),
    };
    
    model.enabled = true;
    model.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}

pub async fn disable_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    
    let model = match sw.models.get_mut(&id) {
        Some(m) => m,
        None => return Json(ApiResponse::err(&format!("Model {} not found", id))),
    };
    
    model.enabled = false;
    model.updated_at = chrono::Utc::now().to_rfc3339();
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::err(&e.to_string()));
    }
    
    Json(ApiResponse::ok(()))
}