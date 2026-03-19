use crate::{AppState, Error, Result};
use tokio::sync::broadcast;

impl AppState {
    pub fn subscribe_events(&self) -> broadcast::Receiver<crate::protocol::AgentEvent> {
        self.event_tx.subscribe()
    }

    pub async fn forward_to_agent(&self, agent_name: &str, _method: &str, _params: &serde_json::Value) -> Result<serde_json::Value> {
        let connections = self.vm_connections.read().await;
        let conn = connections.get(agent_name)
            .ok_or_else(|| Error::NotFound(format!("Agent {} not connected", agent_name)))?;
        
        if !conn.connected {
            return Err(Error::ConnectionLost(format!("Agent {} is not connected", agent_name)));
        }
        
        let _addr = format!("{}:50051", conn.agent_ip);
        drop(connections);
        
        let _client = self.grpc_client.get_or_connect(agent_name, &_addr).await
            .map_err(|e| Error::Internal(e.to_string()))?;
        
        Ok(serde_json::json!({}))
    }
}