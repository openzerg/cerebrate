use async_trait::async_trait;
use super::protocol::RpcError;

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError>;
}