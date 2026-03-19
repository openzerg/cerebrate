use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use crate::api::types::{AgentInfo, CreateAgentRequest, BindModelRequest};
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("agent.list", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let connections = state.vm_connections.read().await;
            
            let agents: Vec<AgentInfo> = sw.agents.iter().map(|(name, agent)| {
                let online = connections.get(name).map(|c| c.connected).unwrap_or(false);
                let model_name = agent.model_id.as_ref()
                    .and_then(|id| sw.models.get(id))
                    .map(|m| m.name.clone());
                AgentInfo {
                    name: name.clone(),
                    enabled: agent.enabled,
                    container_ip: agent.container_ip.clone(),
                    host_ip: agent.host_ip.clone(),
                    forgejo_username: agent.forgejo_username.clone(),
                    online,
                    model_id: agent.model_id.clone(),
                    model_name,
                    internal_token: agent.internal_token.clone(),
                }
            }).collect();
            
            Ok(serde_json::to_value(agents).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' parameter"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let agent = sw.agents.get(&p.name)
                .ok_or_else(|| RpcError::not_found(&format!("Agent '{}'", p.name)))?;
            
            let online = state.vm_connections.read().await
                .get(&p.name).map(|c| c.connected).unwrap_or(false);
            let model_name = agent.model_id.as_ref()
                .and_then(|id| sw.models.get(id))
                .map(|m| m.name.clone());
            
            Ok(serde_json::to_value(AgentInfo {
                name: p.name,
                enabled: agent.enabled,
                container_ip: agent.container_ip.clone(),
                host_ip: agent.host_ip.clone(),
                forgejo_username: agent.forgejo_username.clone(),
                online,
                model_id: agent.model_id.clone(),
                model_name,
                internal_token: agent.internal_token.clone(),
            }).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.create", move |params| {
        let state = state_clone.clone();
        async move {
            let req: CreateAgentRequest = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.agents.contains_key(&req.name) {
                return Err(RpcError::already_exists(&format!("Agent '{}'", req.name)));
            }
            
            let agent_num = sw.agents.len() + 1;
            let now = chrono::Utc::now().to_rfc3339();
            let internal_token = uuid::Uuid::new_v4().to_string();
            let container_ip = format!("{}.{}.2", sw.defaults.container_subnet_base, agent_num);
            let host_ip = format!("{}.{}.1", sw.defaults.container_subnet_base, agent_num);
            
            let agent = crate::models::Agent {
                enabled: true,
                container_ip: container_ip.clone(),
                host_ip: host_ip.clone(),
                forgejo_username: req.forgejo_username.clone(),
                internal_token: internal_token.clone(),
                model_id: None,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.agents.insert(req.name.clone(), agent);
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.apply_tx.send(());
            
            Ok(serde_json::to_value(AgentInfo {
                name: req.name,
                enabled: true,
                container_ip,
                host_ip,
                forgejo_username: req.forgejo_username,
                online: false,
                model_id: None,
                model_name: None,
                internal_token,
            }).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.agents.remove(&p.name).is_none() {
                return Err(RpcError::not_found(&format!("Agent '{}'", p.name)));
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.apply_tx.send(());
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.enable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let agent = sw.agents.get_mut(&p.name)
                .ok_or_else(|| RpcError::not_found(&format!("Agent '{}'", p.name)))?;
            
            agent.enabled = true;
            agent.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.apply_tx.send(());
            
            Ok(serde_json::json!({"enabled": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.disable", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let agent = sw.agents.get_mut(&p.name)
                .ok_or_else(|| RpcError::not_found(&format!("Agent '{}'", p.name)))?;
            
            agent.enabled = false;
            agent.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.apply_tx.send(());
            
            Ok(serde_json::json!({"disabled": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.bind_model", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String, model_id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' or 'model_id' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let model = sw.models.get(&p.model_id)
                .ok_or_else(|| RpcError::not_found(&format!("Model '{}'", p.model_id)))?
                .clone();
            
            let agent = sw.agents.get_mut(&p.name)
                .ok_or_else(|| RpcError::not_found(&format!("Agent '{}'", p.name)))?;
            
            let host_ip = agent.host_ip.clone();
            let internal_token = agent.internal_token.clone();
            
            agent.model_id = Some(p.model_id.clone());
            agent.updated_at = chrono::Utc::now().to_rfc3339();
            let enabled = agent.enabled;
            let container_ip = agent.container_ip.clone();
            let forgejo_username = agent.forgejo_username.clone();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let config_update = crate::protocol::AgentEventMessage::ConfigUpdate {
                llm_base_url: Some(format!("http://{}:17534", host_ip)),
                llm_api_key: Some(internal_token.clone()),
                llm_model: Some(model.name.clone()),
            };
            
            let host_event = crate::protocol::HostEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                event: config_update,
            };
            
            let _ = state.event_tx.send(crate::protocol::AgentEvent {
                event: crate::protocol::AgentEventType::StatusUpdate,
                agent_name: p.name.clone(),
                timestamp: chrono::Utc::now(),
                data: Some(serde_json::to_value(&host_event).unwrap_or(serde_json::Value::Null)),
            });
            
            let online = state.vm_connections.read().await
                .get(&p.name).map(|c| c.connected).unwrap_or(false);
            
            Ok(serde_json::to_value(AgentInfo {
                name: p.name,
                enabled,
                container_ip,
                host_ip,
                forgejo_username,
                online,
                model_id: Some(p.model_id),
                model_name: Some(model.name),
                internal_token,
            }).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("agent.unbind_model", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'name' parameter"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let agent = sw.agents.get_mut(&p.name)
                .ok_or_else(|| RpcError::not_found(&format!("Agent '{}'", p.name)))?;
            
            agent.model_id = None;
            agent.updated_at = chrono::Utc::now().to_rfc3339();
            let enabled = agent.enabled;
            let container_ip = agent.container_ip.clone();
            let host_ip = agent.host_ip.clone();
            let forgejo_username = agent.forgejo_username.clone();
            let internal_token = agent.internal_token.clone();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let online = state.vm_connections.read().await
                .get(&p.name).map(|c| c.connected).unwrap_or(false);
            
            Ok(serde_json::to_value(AgentInfo {
                name: p.name,
                enabled,
                container_ip,
                host_ip,
                forgejo_username,
                online,
                model_id: None,
                model_name: None,
                internal_token,
            }).unwrap())
        }
    }).await;
}