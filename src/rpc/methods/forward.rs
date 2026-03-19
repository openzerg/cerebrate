use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::rpc::protocol::RpcRequest;
use crate::AppState;
use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::oneshot;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    register_session_methods(registry, state.clone()).await;
    register_process_methods(registry, state.clone()).await;
    register_task_methods(registry, state.clone()).await;
    register_message_methods(registry, state.clone()).await;
}

async fn register_session_methods(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("session.list", move |params| {
        let state = state_clone.clone();
        async move {
            let p: AgentParam = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'agent'"))?;
            forward_to_agent(&state, &p.agent, "session.list", None).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("session.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "session.get", Some(serde_json::json!({"id": p.id}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("session.messages", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String, #[serde(default)] offset: Option<usize>, #[serde(default)] limit: Option<usize> }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "session.messages", Some(serde_json::json!({"id": p.id, "offset": p.offset, "limit": p.limit}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("session.chat", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String, content: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "session.chat", Some(serde_json::json!({"id": p.id, "content": p.content}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("session.interrupt", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String, message: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "session.interrupt", Some(serde_json::json!({"id": p.id, "message": p.message}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("session.context", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "session.context", Some(serde_json::json!({"id": p.id}))).await
        }
    }).await;
}

async fn register_process_methods(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("process.list", move |params| {
        let state = state_clone.clone();
        async move {
            let p: AgentParam = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'agent'"))?;
            forward_to_agent(&state, &p.agent, "process.list", None).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("process.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "process.get", Some(serde_json::json!({"id": p.id}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("process.output", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String, #[serde(default)] stream: Option<String>, #[serde(default)] offset: Option<usize>, #[serde(default)] limit: Option<usize> }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "process.output", Some(serde_json::json!({"id": p.id, "stream": p.stream, "offset": p.offset, "limit": p.limit}))).await
        }
    }).await;
}

async fn register_task_methods(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("task.list", move |params| {
        let state = state_clone.clone();
        async move {
            let p: AgentParam = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'agent'"))?;
            forward_to_agent(&state, &p.agent, "task.list", None).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("task.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "task.get", Some(serde_json::json!({"id": p.id}))).await
        }
    }).await;
}

async fn register_message_methods(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("message.send", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, content: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "message.send", Some(serde_json::json!({"content": p.content}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("message.remind", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { agent: String, message: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            forward_to_agent(&state, &p.agent, "message.remind", Some(serde_json::json!({"message": p.message}))).await
        }
    }).await;

    let state_clone = state.clone();
    registry.register("activity.list", move |params| {
        let state = state_clone.clone();
        async move {
            let p: AgentParam = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'agent'"))?;
            forward_to_agent(&state, &p.agent, "activity.list", None).await
        }
    }).await;
}

#[derive(Deserialize)]
struct AgentParam {
    agent: String,
}

async fn forward_to_agent(
    state: &AppState,
    agent_name: &str,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, RpcError> {
    // Get agent connection and rpc_tx
    let rpc_tx = {
        let connections = state.vm_connections.read().await;
        let agent_conn = connections.get(agent_name)
            .ok_or_else(|| RpcError::agent_not_found(agent_name))?;
        
        if !agent_conn.connected {
            return Err(RpcError::agent_not_found(agent_name));
        }
        
        agent_conn.rpc_tx.clone()
            .ok_or_else(|| RpcError::internal_error("Agent RPC channel not available"))?
    };
    
    // Create request ID and pending response channel
    let request_id = chrono::Utc::now().timestamp();
    let (tx, rx) = oneshot::channel();
    
    {
        let mut pending = state.pending_rpc_requests.write().await;
        pending.insert(request_id.to_string(), tx);
    }
    
    // Create and send RPC request
    let request = RpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(request_id),
        method: method.to_string(),
        params,
    };
    
    let json = request.to_json()
        .map_err(|e| RpcError::internal_error(e.message))?;
    
    rpc_tx.send(json)
        .map_err(|_| RpcError::internal_error("Failed to send RPC request to agent"))?;
    
    // Wait for response with timeout
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        rx
    ).await
        .map_err(|_| RpcError::internal_error("RPC request timeout"))?
        .map_err(|_| RpcError::internal_error("RPC response channel closed"))?;
    
    // Return result or error
    if let Some(result) = response.result {
        Ok(result)
    } else if let Some(error) = response.error {
        Err(error)
    } else {
        Err(RpcError::internal_error("Empty RPC response"))
    }
}