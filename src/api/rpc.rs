use std::sync::Arc;
use axum::{
    extract::{State, ws::{WebSocketUpgrade, WebSocket, Message}},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use crate::AppState;
use crate::rpc::protocol::{RpcRequest, RpcResponse};

pub async fn rpc_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_rpc_socket(socket, state))
}

async fn handle_rpc_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();
    let registry = state.rpc_registry.clone();

    tracing::info!("RPC client connected");

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match RpcRequest::parse(&text) {
                    Ok(request) => {
                        tracing::debug!("RPC request: {}", request.method);
                        let response = registry.dispatch(request).await;
                        if let Ok(json) = response.to_json() {
                            if tx.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        let response = RpcResponse::error(None, error);
                        if let Ok(json) = response.to_json() {
                            if tx.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                if tx.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::error!("RPC WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    tracing::info!("RPC client disconnected");
}