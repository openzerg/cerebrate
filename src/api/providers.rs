use std::sync::Arc;
use axum::{
    Json, extract::{Path, State},
};
use crate::AppState;
use super::types::{ApiResponse, ProviderInfo, ModelInfo};
use crate::models::{CreateProviderRequest, CreateModelRequest, Provider, Model};

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

pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<ModelInfo>>> {
    let sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let models: Vec<ModelInfo> = sw.models.values().filter_map(|m| {
        sw.providers.get(&m.provider_id).map(|p| ModelInfo {
            id: m.id.clone(),
            name: m.name.clone(),
            provider_id: m.provider_id.clone(),
            provider_name: p.name.clone(),
            model_name: m.model_name.clone(),
            enabled: m.enabled,
            created_at: m.created_at.clone(),
        })
    }).collect();
    
    Json(ApiResponse::success(models))
}

pub async fn create_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateModelRequest>,
) -> Json<ApiResponse<ModelInfo>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    let provider = match sw.providers.get(&req.provider_id) {
        Some(p) => p.clone(),
        None => return Json(ApiResponse::error(format!("Provider '{}' not found", req.provider_id))),
    };
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let model = Model {
        id: id.clone(),
        name: req.name.clone(),
        provider_id: req.provider_id.clone(),
        model_name: req.model_name.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    
    sw.models.insert(id.clone(), model);
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(ModelInfo {
        id,
        name: req.name,
        provider_id: req.provider_id,
        provider_name: provider.name,
        model_name: req.model_name,
        enabled: true,
        created_at: now,
    }))
}

pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    if sw.models.remove(&id).is_none() {
        return Json(ApiResponse::error(format!("Model '{}' not found", id)));
    }
    
    for agent in sw.agents.values_mut() {
        if agent.model_id.as_ref() == Some(&id) {
            agent.model_id = None;
        }
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Model '{}' deleted", id)))
}

pub async fn enable_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.models.get_mut(&id) {
        Some(m) => {
            m.enabled = true;
            m.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Model '{}' not found", id))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Model '{}' enabled", id)))
}

pub async fn disable_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<String>> {
    let mut sw = match state.state_manager.load().await {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    
    match sw.models.get_mut(&id) {
        Some(m) => {
            m.enabled = false;
            m.updated_at = chrono::Utc::now().to_rfc3339();
        }
        None => return Json(ApiResponse::error(format!("Model '{}' not found", id))),
    }
    
    if let Err(e) = state.state_manager.save(&sw).await {
        return Json(ApiResponse::error(e.to_string()));
    }
    
    Json(ApiResponse::success(format!("Model '{}' disabled", id)))
}