use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::protocol::{RpcRequest, RpcResponse, RpcError};
use std::future::Future;
use std::pin::Pin;

type HandlerFn = Box<dyn Fn(Option<serde_json::Value>) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send>> + Send + Sync>;

pub struct RpcRegistry {
    handlers: Arc<RwLock<HashMap<String, HandlerFn>>>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value, RpcError>> + Send + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            method.to_string(),
            Box::new(move |params| Box::pin(handler(params))),
        );
    }

    pub async fn dispatch(&self, request: RpcRequest) -> RpcResponse {
        let handlers = self.handlers.read().await;
        
        match handlers.get(&request.method) {
            Some(handler) => {
                match handler(request.params).await {
                    Ok(result) => RpcResponse::success(request.id, result),
                    Err(error) => RpcResponse::error(request.id, error),
                }
            }
            None => RpcResponse::error(request.id, RpcError::method_not_found()),
        }
    }

    pub async fn list_methods(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
}

impl Default for RpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RpcRegistry {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
        }
    }
}