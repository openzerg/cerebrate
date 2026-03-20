use std::path::Path;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

use super::types::{Instance, InstanceState, InstanceConfig, Operation};

const DEFAULT_SOCKET_PATH: &str = "/var/lib/incus/unix.socket";

#[derive(Clone)]
pub struct IncusClient {
    socket_path: String,
    http_client: Client,
}

impl IncusClient {
    pub fn new() -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
            http_client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        }
    }

    pub fn with_socket(socket_path: &Path) -> Self {
        Self {
            socket_path: socket_path.to_string_lossy().to_string(),
            http_client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        }
    }

    pub async fn ping(&self) -> crate::Result<bool> {
        match self.get::<serde_json::Value>("/1.0").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn list_instances(&self) -> crate::Result<Vec<Instance>> {
        let urls: Vec<String> = self.get("/1.0/instances").await?;
        let mut result = Vec::new();
        for url in urls {
            if let Some(name) = url.strip_prefix("/1.0/instances/") {
                match self.get_instance(name).await {
                    Ok(instance) => result.push(instance),
                    Err(e) => tracing::warn!("Failed to get instance {}: {}", name, e),
                }
            }
        }
        Ok(result)
    }

    pub async fn get_instance(&self, name: &str) -> crate::Result<Instance> {
        self.get(&format!("/1.0/instances/{}", name)).await
    }

    pub async fn get_instance_state(&self, name: &str) -> crate::Result<InstanceState> {
        self.get(&format!("/1.0/instances/{}/state", name)).await
    }

    pub async fn create_instance(&self, config: &InstanceConfig) -> crate::Result<Operation> {
        self.post("/1.0/instances", config).await
    }

    pub async fn start_instance(&self, name: &str) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "action": "start",
            "timeout": 30,
        });
        self.put(&format!("/1.0/instances/{}/state", name), &body).await
    }

    pub async fn stop_instance(&self, name: &str, force: bool) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "action": "stop",
            "force": force,
            "timeout": 30,
        });
        self.put(&format!("/1.0/instances/{}/state", name), &body).await
    }

    pub async fn restart_instance(&self, name: &str, force: bool) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "action": "restart",
            "force": force,
            "timeout": 30,
        });
        self.put(&format!("/1.0/instances/{}/state", name), &body).await
    }

    pub async fn delete_instance(&self, name: &str) -> crate::Result<Operation> {
        self.delete(&format!("/1.0/instances/{}", name)).await
    }

    pub async fn create_snapshot(&self, instance_name: &str, snapshot_name: &str, stateful: bool) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "name": snapshot_name,
            "stateful": stateful,
        });
        self.post(&format!("/1.0/instances/{}/snapshots", instance_name), &body).await
    }

    pub async fn list_snapshots(&self, instance_name: &str) -> crate::Result<Vec<Instance>> {
        let urls: Vec<String> = self.get(&format!("/1.0/instances/{}/snapshots", instance_name)).await?;
        let mut result = Vec::new();
        for url in urls {
            if let Some(name) = url.strip_prefix("/1.0/instances/") {
                match self.get(&format!("/1.0/instances/{}", name)).await {
                    Ok(snapshot) => result.push(snapshot),
                    Err(e) => tracing::warn!("Failed to get snapshot {}: {}", name, e),
                }
            }
        }
        Ok(result)
    }

    pub async fn delete_snapshot(&self, instance_name: &str, snapshot_name: &str) -> crate::Result<Operation> {
        self.delete(&format!("/1.0/instances/{}/snapshots/{}", instance_name, snapshot_name)).await
    }

    pub async fn restore_snapshot(&self, instance_name: &str, snapshot_name: &str) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "restore": snapshot_name,
        });
        self.put(&format!("/1.0/instances/{}", instance_name), &body).await
    }

    pub async fn exec(&self, name: &str, command: &[&str], environment: Option<serde_json::Value>) -> crate::Result<Operation> {
        let body = serde_json::json!({
            "command": command,
            "environment": environment.unwrap_or(serde_json::json!({})),
            "wait-for-websocket": false,
            "interactive": false,
        });
        self.post(&format!("/1.0/instances/{}/exec", name), &body).await
    }

    pub async fn get_operation(&self, operation_id: &str) -> crate::Result<Operation> {
        let id = operation_id.strip_prefix("/1.0/operations/").unwrap_or(operation_id);
        self.get(&format!("/1.0/operations/{}", id)).await
    }

    pub async fn wait_operation(&self, operation_id: &str, timeout: u32) -> crate::Result<Operation> {
        let id = operation_id.strip_prefix("/1.0/operations/").unwrap_or(operation_id);
        self.get(&format!("/1.0/operations/{}/wait?timeout={}", id, timeout)).await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> crate::Result<T> {
        let output = self.incus_command(&["query", path]).await?;
        let result: T = serde_json::from_str(&output)
            .map_err(|e| crate::Error::Config(format!("Failed to parse response: {}", e)))?;
        Ok(result)
    }

    async fn post<T: Serialize>(&self, path: &str, body: &T) -> crate::Result<Operation> {
        let json = serde_json::to_string(body)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize: {}", e)))?;
        let output = self.incus_command(&["query", "-X", "POST", "-d", &json, path]).await?;
        let result: Operation = serde_json::from_str(&output)
            .map_err(|e| crate::Error::Config(format!("Failed to parse response: {}", e)))?;
        Ok(result)
    }

    async fn put<T: Serialize>(&self, path: &str, body: &T) -> crate::Result<Operation> {
        let json = serde_json::to_string(body)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize: {}", e)))?;
        let output = self.incus_command(&["query", "-X", "PUT", "-d", &json, path]).await?;
        let result: Operation = serde_json::from_str(&output)
            .map_err(|e| crate::Error::Config(format!("Failed to parse response: {}", e)))?;
        Ok(result)
    }

    async fn delete(&self, path: &str) -> crate::Result<Operation> {
        let output = self.incus_command(&["query", "-X", "DELETE", path]).await?;
        let result: Operation = serde_json::from_str(&output)
            .map_err(|e| crate::Error::Config(format!("Failed to parse response: {}", e)))?;
        Ok(result)
    }

    async fn incus_command(&self, args: &[&str]) -> crate::Result<String> {
        let mut cmd = tokio::process::Command::new("incus");
        cmd.args(args);

        let output = cmd.output().await
            .map_err(|e| crate::Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Config(format!("Incus command failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for IncusClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = IncusClient::new();
        assert_eq!(client.socket_path, DEFAULT_SOCKET_PATH);
    }
}