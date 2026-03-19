use std::sync::Arc;
use axum::extract::{State, ws::WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt};
use crate::{AppState, VmConnection};
use crate::protocol::{Message, VmConnect, VmHeartbeat, VmSkillResult, AgentEvent, AgentEventType};

pub async fn event_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_event_ws(socket, state))
}

async fn handle_event_ws(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();
    let mut event_rx = state.event_tx.subscribe();

    tracing::info!("Event subscriber connected");

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Ok(agent_event) => {
                        let msg = Message::AgentEvent(agent_event);
                        match msg.to_json() {
                            Ok(json) => {
                                if tx.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            msg = rx.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        if tx.send(axum::extract::ws::Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::info!("Event subscriber disconnected");
}

pub async fn vm_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_vm_ws(socket, state))
}

async fn handle_vm_ws(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();
    let mut agent_name: Option<String> = None;
    let mut event_rx = state.event_tx.subscribe();

    tracing::info!("VM WebSocket connection");

    loop {
        tokio::select! {
            msg = rx.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        match Message::from_json(&text) {
                            Ok(message) => {
                                match message {
                                    Message::VmConnect(connect) => {
                                        match handle_vm_connect(&state, &connect).await {
                                            Ok(name) => agent_name = Some(name.clone()),
                                            Err(e) => {
                                                tracing::error!("VM connect error: {}", e);
                                                let _ = tx.send(axum::extract::ws::Message::Close(None)).await;
                                                break;
                                            }
                                        }
                                    }
                                    Message::VmHeartbeat(heartbeat) => {
                                        handle_vm_heartbeat(&state, &heartbeat).await;
                                    }
                                    Message::VmStatusReport(report) => {
                                        let _ = state.event_tx.send(AgentEvent {
                                            event: AgentEventType::StatusUpdate,
                                            agent_name: report.agent_name.clone(),
                                            timestamp: chrono::Utc::now(),
                                            data: Some(serde_json::to_value(&report.data).unwrap_or(serde_json::Value::Null)),
                                        });
                                    }
                                    Message::VmSkillResult(result) => {
                                        handle_vm_skill_result(&state, &result).await;
                                    }
                                    Message::VmEventAck(ack) => {
                                        tracing::debug!("Event acked: {}", ack.event_id);
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) => break,
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        if tx.send(axum::extract::ws::Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            event = event_rx.recv() => {
                if let Some(ref name) = agent_name {
                    if let Ok(agent_event) = event {
                        if agent_event.agent_name == *name {
                            if let Some(data) = &agent_event.data {
                                if let Ok(host_event) = serde_json::from_value::<crate::protocol::HostEvent>(data.clone()) {
                                    let msg = Message::HostEvent(host_event);
                                    if let Ok(json) = msg.to_json() {
                                        if tx.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(name) = agent_name {
        let mut connections = state.vm_connections.write().await;
        connections.remove(&name);
        drop(connections);
        
        let _ = state.event_tx.send(AgentEvent {
            event: AgentEventType::Disconnected,
            agent_name: name.clone(),
            timestamp: chrono::Utc::now(),
            data: None,
        });
        tracing::info!("Agent {} disconnected", name);
    }
}

async fn handle_vm_connect(state: &AppState, connect: &VmConnect) -> crate::Result<String> {
    let sw = state.state_manager.load().await?;
    let agent = sw.agents.get(&connect.agent_name)
        .ok_or_else(|| crate::Error::AgentNotFound(connect.agent_name.clone()))?;

    if agent.internal_token != connect.internal_token {
        return Err(crate::Error::InvalidToken);
    }

    {
        let mut connections = state.vm_connections.write().await;
        connections.insert(connect.agent_name.clone(), VmConnection {
            agent_name: connect.agent_name.clone(),
            connected: true,
            last_heartbeat: chrono::Utc::now(),
            rpc_tx: None,
        });
    }

    let _ = state.event_tx.send(AgentEvent {
        event: AgentEventType::Connected,
        agent_name: connect.agent_name.clone(),
        timestamp: chrono::Utc::now(),
        data: None,
    });

    tracing::info!("Agent {} connected", connect.agent_name);
    Ok(connect.agent_name.clone())
}

async fn handle_vm_heartbeat(state: &AppState, heartbeat: &VmHeartbeat) {
    let mut connections = state.vm_connections.write().await;
    if let Some(conn) = connections.get_mut(&heartbeat.agent_name) {
        conn.last_heartbeat = chrono::Utc::now();
        conn.connected = true;
    }
}

async fn handle_vm_skill_result(state: &AppState, result: &VmSkillResult) {
    use crate::models::InvokeToolResponse;
    
    let mut pending = state.pending_tool_results.write().await;
    if let Some(tx) = pending.remove(&result.skill_id) {
        let response = InvokeToolResponse {
            success: result.success,
            output: result.output.clone(),
            error: result.error.clone(),
        };
        let _ = tx.send(response);
    }
}