use std::sync::Arc;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use super::protocol::{RpcRequest, RpcResponse};
use super::registry::RpcRegistry;

pub async fn rpc_ws_handler(
    ws: WebSocketUpgrade,
    State(registry): State<Arc<RpcRegistry>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_rpc_socket(socket, registry))
}

async fn handle_rpc_socket(socket: WebSocket, registry: Arc<RpcRegistry>) {
    let (mut tx, mut rx) = socket.split();

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match RpcRequest::parse(&text) {
                    Ok(request) => {
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
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}