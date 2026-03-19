use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use crate::api::types::{ProviderInfo, ModelInfo};
use crate::models::{CreateProviderRequest, CreateModelRequest, Provider, Model};
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("llm.provider.list", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let providers: Vec<ProviderInfo> = sw.providers.values().map(|p| ProviderInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                provider_type: p.provider_type.as_str().to_string(),
                base_url: p.base_url.clone(),
                enabled: p.enabled,
                created_at: p.created_at.clone(),
            }).collect();
            
            Ok(serde_json::to_value(providers).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.provider.create", move |params| {
        let state = state_clone.clone();
        async move {
            let req: CreateProviderRequest = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
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
                updated_at: now.clone(),
            };
            
            sw.providers.insert(id.clone(), provider);
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::to_value(ProviderInfo {
                id,
                name: req.name,
                provider_type: req.provider_type.as_str().to_string(),
                base_url: req.base_url,
                enabled: true,
                created_at: now,
            }).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.provider.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.providers.remove(&p.id).is_none() {
                return Err(RpcError::not_found(&format!("Provider '{}'", p.id)));
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.provider.enable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let provider = sw.providers.get_mut(&p.id)
                .ok_or_else(|| RpcError::not_found(&format!("Provider '{}'", p.id)))?;
            
            provider.enabled = true;
            provider.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"enabled": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.provider.disable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let provider = sw.providers.get_mut(&p.id)
                .ok_or_else(|| RpcError::not_found(&format!("Provider '{}'", p.id)))?;
            
            provider.enabled = false;
            provider.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"disabled": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.model.list", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
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
            
            Ok(serde_json::to_value(models).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.model.create", move |params| {
        let state = state_clone.clone();
        async move {
            let req: CreateModelRequest = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let provider = sw.providers.get(&req.provider_id)
                .ok_or_else(|| RpcError::not_found(&format!("Provider '{}'", req.provider_id)))?
                .clone();
            
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
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::to_value(ModelInfo {
                id,
                name: req.name,
                provider_id: req.provider_id,
                provider_name: provider.name,
                model_name: req.model_name,
                enabled: true,
                created_at: now,
            }).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.model.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.models.remove(&p.id).is_none() {
                return Err(RpcError::not_found(&format!("Model '{}'", p.id)));
            }
            
            for agent in sw.agents.values_mut() {
                if agent.model_id.as_ref() == Some(&p.id) {
                    agent.model_id = None;
                }
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.model.enable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let model = sw.models.get_mut(&p.id)
                .ok_or_else(|| RpcError::not_found(&format!("Model '{}'", p.id)))?;
            
            model.enabled = true;
            model.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"enabled": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("llm.model.disable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let model = sw.models.get_mut(&p.id)
                .ok_or_else(|| RpcError::not_found(&format!("Model '{}'", p.id)))?;
            
            model.enabled = false;
            model.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"disabled": true}))
        }
    }).await;
}