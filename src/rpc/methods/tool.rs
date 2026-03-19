use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use crate::api::tools::ToolInfo;
use crate::models::CreateToolRequest;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("tool.list", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            let tools: Vec<ToolInfo> = sw.tools.values().cloned().map(ToolInfo::from).collect();
            Ok(serde_json::to_value(tools).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let tool = sw.tools.get(&p.slug)
                .ok_or_else(|| RpcError::not_found(&format!("Tool '{}'", p.slug)))?;
            
            Ok(serde_json::to_value(ToolInfo::from(tool.clone())).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.clone", move |params| {
        let state = state_clone.clone();
        async move {
            let req: CreateToolRequest = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.tools.contains_key(&req.slug) {
                return Err(RpcError::already_exists(&format!("Tool '{}'", req.slug)));
            }
            
            if !sw.agents.contains_key(&req.author_agent) {
                return Err(RpcError::not_found(&format!("Agent '{}'", req.author_agent)));
            }
            
            state.tool_manager.clone_tool(&req.slug, &req.forgejo_repo).await
                .map_err(|e| RpcError::internal_error(format!("Failed to clone tool: {}", e)))?;
            
            let metadata = state.tool_manager.parse_tool_md(&req.slug)
                .map_err(|e| {
                    let _ = state.tool_manager.delete_tool(&req.slug);
                    RpcError::internal_error(format!("Failed to parse TOOL.md: {}", e))
                })?;
            
            let git_commit = state.tool_manager.get_git_commit(&req.slug).await
                .map_err(|e| {
                    let _ = state.tool_manager.delete_tool(&req.slug);
                    RpcError::internal_error(format!("Failed to get git commit: {}", e))
                })?;
            
            let now = chrono::Utc::now().to_rfc3339();
            let tool = crate::models::Tool {
                slug: req.slug.clone(),
                name: metadata.name,
                version: metadata.version,
                description: metadata.description,
                forgejo_repo: req.forgejo_repo,
                git_commit,
                entrypoint: metadata.entrypoint,
                input_schema: metadata.input_schema,
                output_schema: metadata.output_schema,
                author_agent: req.author_agent.clone(),
                allowed_agents: vec![req.author_agent],
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.tools.insert(req.slug.clone(), tool.clone());
            state.state_manager.save(&sw).await
                .map_err(|e| {
                    let _ = state.tool_manager.delete_tool(&req.slug);
                    RpcError::internal_error(e.to_string())
                })?;
            
            Ok(serde_json::to_value(ToolInfo::from(tool)).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.pull", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.tools.contains_key(&p.slug) {
                return Err(RpcError::not_found(&format!("Tool '{}'", p.slug)));
            }
            
            let new_commit = state.tool_manager.pull_tool(&p.slug).await
                .map_err(|e| RpcError::internal_error(format!("Failed to pull tool: {}", e)))?;
            
            let metadata = state.tool_manager.parse_tool_md(&p.slug)
                .map_err(|e| RpcError::internal_error(format!("Failed to parse TOOL.md: {}", e)))?;
            
            if let Some(t) = sw.tools.get_mut(&p.slug) {
                t.git_commit = new_commit;
                t.version = metadata.version;
                t.description = metadata.description;
                t.entrypoint = metadata.entrypoint;
                t.input_schema = metadata.input_schema;
                t.output_schema = metadata.output_schema;
                t.updated_at = chrono::Utc::now().to_rfc3339();
                let tool = t.clone();
                let _ = state.state_manager.save(&sw).await;
                return Ok(serde_json::to_value(ToolInfo::from(tool)).unwrap());
            }
            
            Err(RpcError::not_found(&format!("Tool '{}'", p.slug)))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.tools.remove(&p.slug).is_none() {
                return Err(RpcError::not_found(&format!("Tool '{}'", p.slug)));
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.tool_manager.delete_tool(&p.slug).await;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.authorize", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String, agent_name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.agents.contains_key(&p.agent_name) {
                return Err(RpcError::not_found(&format!("Agent '{}'", p.agent_name)));
            }
            
            let tool = sw.tools.get_mut(&p.slug)
                .ok_or_else(|| RpcError::not_found(&format!("Tool '{}'", p.slug)))?;
            
            if !tool.allowed_agents.contains(&p.agent_name) {
                tool.allowed_agents.push(p.agent_name.clone());
                tool.updated_at = chrono::Utc::now().to_rfc3339();
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"authorized": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.revoke", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String, agent_name: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let tool = sw.tools.get_mut(&p.slug)
                .ok_or_else(|| RpcError::not_found(&format!("Tool '{}'", p.slug)))?;
            
            tool.allowed_agents.retain(|a| a != &p.agent_name);
            tool.updated_at = chrono::Utc::now().to_rfc3339();
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"revoked": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.invoke", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { 
                slug: String, 
                input: serde_json::Value,
                #[serde(default)]
                caller: Option<String>,
            }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let tool = sw.tools.get(&p.slug)
                .ok_or_else(|| RpcError::not_found(&format!("Tool '{}'", p.slug)))?
                .clone();
            
            if let Some(caller_name) = &p.caller {
                if !tool.allowed_agents.contains(caller_name) && tool.author_agent != *caller_name {
                    return Err(RpcError::unauthorized());
                }
            }
            
            if !tool.enabled {
                return Err(RpcError::internal_error("Tool is disabled"));
            }
            
            state.tool_manager.invoke_host_tool(&tool, &p.input).await
                .map(|resp| serde_json::to_value(resp).unwrap())
                .map_err(|e| RpcError::internal_error(e.to_string()))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.env.list", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.tools.contains_key(&p.slug) {
                return Err(RpcError::not_found(&format!("Tool '{}'", p.slug)));
            }
            
            let keys = state.tool_manager.list_env(&p.slug).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::to_value(keys).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.env.set", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String, key: String, value: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.tools.contains_key(&p.slug) {
                return Err(RpcError::not_found(&format!("Tool '{}'", p.slug)));
            }
            
            state.tool_manager.set_env(&p.slug, &p.key, &p.value).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"set": true}))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("tool.env.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String, key: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.tools.contains_key(&p.slug) {
                return Err(RpcError::not_found(&format!("Tool '{}'", p.slug)));
            }
            
            state.tool_manager.delete_env(&p.slug, &p.key).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;
}