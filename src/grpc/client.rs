use tonic::transport::{Channel, Uri};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Duration;

use crate::grpc::swarm::swarm_service_client::SwarmServiceClient;

pub struct AgentGrpcClient {
    clients: RwLock<HashMap<String, SwarmServiceClient<Channel>>>,
}

impl AgentGrpcClient {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_connect(&self, agent_name: &str, addr: &str) -> Result<SwarmServiceClient<Channel>, tonic::transport::Error> {
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(agent_name) {
                return Ok(client.clone());
            }
        }
        
        let uri: Uri = format!("http://{}", addr).parse().unwrap();
        let client = SwarmServiceClient::connect(uri).await?;
        
        {
            let mut clients = self.clients.write().await;
            clients.insert(agent_name.to_string(), client.clone());
        }
        
        Ok(client)
    }

    pub async fn remove(&self, agent_name: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(agent_name);
    }
}