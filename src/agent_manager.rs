use crate::incus::IncusClient;
use crate::models::{Agent, Defaults, State};
use crate::{Error, Result};
use std::path::Path;

const AGENT_LABEL_KEY: &str = "user.openzerg.type";
const AGENT_LABEL_VALUE: &str = "agent";

#[derive(Clone)]
pub struct AgentManager {
    data_dir: std::path::PathBuf,
    incus_client: IncusClient,
}

impl AgentManager {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            incus_client: IncusClient::new(),
        }
    }

    pub async fn apply(&self, state: &State) -> Result<()> {
        self.ensure_containers(&state.agents).await?;
        self.cleanup_removed_agents(&state.agents).await?;
        Ok(())
    }

    fn is_agent_container(instance: &crate::incus::Instance) -> bool {
        instance.config
            .as_ref()
            .and_then(|c| c.get(AGENT_LABEL_KEY))
            .map(|v| v == AGENT_LABEL_VALUE)
            .unwrap_or(false)
    }

    async fn ensure_containers(
        &self,
        agents: &std::collections::HashMap<String, Agent>,
    ) -> Result<()> {
        for (name, agent) in agents {
            match self.incus_client.get_instance(name).await {
                Ok(_) => {
                    tracing::info!("Container {} already exists", name);
                    self.update_container_state(name, agent.enabled).await?;
                }
                Err(_) => {
                    tracing::info!("Creating agent container {}", name);
                    self.create_agent_container(name, agent.enabled).await?;
                }
            }
        }
        Ok(())
    }

    async fn create_agent_container(&self, name: &str, enabled: bool) -> Result<()> {
        use crate::incus::InstanceConfig;

        let mut config_map = std::collections::HashMap::new();
        config_map.insert(AGENT_LABEL_KEY.to_string(), AGENT_LABEL_VALUE.to_string());

        let config = InstanceConfig::new(name)
            .with_profiles(vec!["default".to_string()])
            .with_config(config_map);

        let operation = self.incus_client.create_instance(&config).await?;
        tracing::info!("Created container {} (operation: {})", name, operation.id);

        if enabled {
            let wait_result = self.incus_client.wait_operation(&operation.id, 60).await?;
            
            if let Some(err) = wait_result.err {
                if !err.is_empty() {
                    return Err(Error::Config(format!("Failed to create container: {}", err)));
                }
            }
            
            let start_op = self.incus_client.start_instance(name).await?;
            self.incus_client.wait_operation(&start_op.id, 60).await?;
            tracing::info!("Started container {}", name);
        }

        Ok(())
    }

    async fn update_container_state(&self, name: &str, enabled: bool) -> Result<()> {
        let state = self.incus_client.get_instance_state(name).await?;
        
        let is_running = state.status == "Running";
        
        if enabled && !is_running {
            tracing::info!("Starting container {}", name);
            let op = self.incus_client.start_instance(name).await?;
            self.incus_client.wait_operation(&op.id, 60).await?;
        } else if !enabled && is_running {
            tracing::info!("Stopping container {}", name);
            let op = self.incus_client.stop_instance(name, false).await?;
            self.incus_client.wait_operation(&op.id, 60).await?;
        }

        Ok(())
    }

    async fn cleanup_removed_agents(
        &self,
        agents: &std::collections::HashMap<String, Agent>,
    ) -> Result<()> {
        let instances = self.incus_client.list_instances().await?;

        for instance in instances {
            if !Self::is_agent_container(&instance) {
                continue;
            }

            if !agents.contains_key(&instance.name) {
                tracing::info!("Removing agent container {} (no longer in state)", instance.name);
                
                if instance.status == "Running" {
                    let op = self.incus_client.stop_instance(&instance.name, true).await?;
                    self.incus_client.wait_operation(&op.id, 30).await?;
                }

                let op = self.incus_client.delete_instance(&instance.name).await?;
                self.incus_client.wait_operation(&op.id, 30).await?;
            }
        }

        Ok(())
    }

    pub async fn get_container_ip(&self, name: &str) -> Result<Option<String>> {
        let state = self.incus_client.get_instance_state(name).await?;
        
        if let Some(network) = state.network.get("eth0") {
            for addr in &network.addresses {
                if addr.family == "inet" {
                    return Ok(Some(addr.address.clone()));
                }
            }
        }

        Ok(None)
    }

    pub async fn exec_in_container(&self, name: &str, command: &[&str]) -> Result<String> {
        let op = self.incus_client.exec(name, command, None).await?;
        let result = self.incus_client.wait_operation(&op.id, 300).await?;
        
        if let Some(err) = result.err {
            return Err(Error::Config(format!("Exec failed: {}", err)));
        }

        Ok(String::new())
    }

    pub async fn create_snapshot(&self, name: &str, snapshot_name: &str, stateful: bool) -> Result<()> {
        let op = self.incus_client.create_snapshot(name, snapshot_name, stateful).await?;
        self.incus_client.wait_operation(&op.id, 120).await?;
        tracing::info!("Created snapshot {} for {}", snapshot_name, name);
        Ok(())
    }

    pub async fn restore_snapshot(&self, name: &str, snapshot_name: &str) -> Result<()> {
        let op = self.incus_client.restore_snapshot(name, snapshot_name).await?;
        self.incus_client.wait_operation(&op.id, 120).await?;
        tracing::info!("Restored snapshot {} for {}", snapshot_name, name);
        Ok(())
    }

    pub async fn list_snapshots(&self, name: &str) -> Result<Vec<String>> {
        let snapshots = self.incus_client.list_snapshots(name).await?;
        Ok(snapshots.into_iter().map(|s| s.name).collect())
    }

    pub async fn delete_snapshot(&self, name: &str, snapshot_name: &str) -> Result<()> {
        let op = self.incus_client.delete_snapshot(name, snapshot_name).await?;
        self.incus_client.wait_operation(&op.id, 30).await?;
        tracing::info!("Deleted snapshot {} for {}", snapshot_name, name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_manager_new() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        assert_eq!(manager.data_dir, dir.path());
    }

    #[test]
    fn test_agent_label_constants() {
        assert_eq!(AGENT_LABEL_KEY, "user.openzerg.type");
        assert_eq!(AGENT_LABEL_VALUE, "agent");
    }
}