use crate::incus::IncusClient;
use crate::models::{CheckpointMeta, State};
use crate::state::StateManager;
use crate::{Error, Result};
use std::path::Path;

const MAX_CHECKPOINTS_PER_AGENT: usize = 10;

pub struct CheckpointManager {
    state_manager: StateManager,
    incus_client: IncusClient,
    checkpoints_dir: std::path::PathBuf,
}

impl CheckpointManager {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            state_manager: StateManager::new(data_dir),
            incus_client: IncusClient::new(),
            checkpoints_dir: data_dir.join("checkpoints"),
        }
    }

    pub async fn create_checkpoint(&self, agent_name: &str, description: &str) -> Result<String> {
        let state = self.state_manager.load().await?;
        if !state.agents.contains_key(agent_name) {
            return Err(Error::AgentNotFound(agent_name.to_string()));
        }

        let count = self.state_manager.count_checkpoints(agent_name).await?;
        if count >= MAX_CHECKPOINTS_PER_AGENT {
            return Err(Error::Validation(format!(
                "Agent {} already has {} checkpoints (max {}). Please delete old checkpoints first.",
                agent_name, count, MAX_CHECKPOINTS_PER_AGENT
            )));
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let short_uuid = &uuid::Uuid::new_v4().to_string()[..8];
        let checkpoint_id = format!("cp_{}_{}", timestamp, short_uuid);

        let checkpoint_dir = self.checkpoints_dir.join(&checkpoint_id);
        tokio::fs::create_dir_all(&checkpoint_dir).await?;

        let state_path = checkpoint_dir.join("state.json");
        let state_content = serde_json::to_string_pretty(&state)?;
        tokio::fs::write(&state_path, state_content).await?;

        let instance_state = self.incus_client.get_instance_state(agent_name).await?;
        let stateful = false;

        tracing::info!("Creating {} snapshot for {}...", 
            if stateful { "stateful" } else { "stateless" }, 
            agent_name
        );
        
        let op = self.incus_client.create_snapshot(agent_name, &checkpoint_id, stateful).await?;
        let result = self.incus_client.wait_operation(&op.id, 120).await?;
        if let Some(err) = result.err {
            if !err.is_empty() {
                return Err(Error::Config(format!("Failed to create snapshot: {}", err)));
            }
        }

        let meta = CheckpointMeta {
            id: checkpoint_id.clone(),
            agent_name: agent_name.to_string(),
            description: description.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            snapshot_ref: format!("incus-snapshot:{}", checkpoint_id),
        };

        let meta_path = checkpoint_dir.join("meta.json");
        let meta_content = serde_json::to_string_pretty(&meta)?;
        tokio::fs::write(&meta_path, meta_content).await?;

        Ok(checkpoint_id)
    }

    pub async fn rollback(&self, agent_name: &str, checkpoint_id: &str) -> Result<()> {
        let (checkpoint_state, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        if meta.agent_name != agent_name {
            return Err(Error::Validation(format!(
                "Checkpoint {} is for agent {}, not {}",
                checkpoint_id, meta.agent_name, agent_name
            )));
        }

        let mut current_state = self.state_manager.load().await?;

        tracing::info!("Restoring snapshot {} for {}...", checkpoint_id, agent_name);
        
        let op = self.incus_client.restore_snapshot(agent_name, checkpoint_id).await?;
        let result = self.incus_client.wait_operation(&op.id, 120).await?;
        if let Some(err) = result.err {
            if !err.is_empty() {
                return Err(Error::Config(format!("Failed to restore snapshot: {}", err)));
            }
        }

        if let Some(agent_config) = checkpoint_state.agents.get(agent_name) {
            current_state.agents.insert(agent_name.to_string(), agent_config.clone());
        }

        self.state_manager.save(&current_state).await?;

        tracing::info!("Rollback complete for {}", agent_name);
        Ok(())
    }

    pub async fn clone(&self, checkpoint_id: &str, new_agent_name: &str) -> Result<()> {
        let (checkpoint_state, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        let mut current_state = self.state_manager.load().await?;
        if current_state.agents.contains_key(new_agent_name) {
            return Err(Error::AgentAlreadyExists(new_agent_name.to_string()));
        }

        let source_agent = checkpoint_state.agents.get(&meta.agent_name)
            .ok_or_else(|| Error::AgentNotFound(meta.agent_name.clone()))?;

        let now = chrono::Utc::now().to_rfc3339();
        let agent_num = current_state.agents.len() + 1;
        let defaults = &current_state.defaults;

        let new_agent = crate::models::Agent {
            enabled: true,
            container_ip: format!("{}.{}.2", defaults.container_subnet_base, agent_num),
            host_ip: format!("{}.{}.1", defaults.container_subnet_base, agent_num),
            forgejo_username: Some(new_agent_name.to_string()),
            internal_token: uuid::Uuid::new_v4().to_string(),
            model_id: None,
            created_at: now.clone(),
            updated_at: now,
        };

        tracing::info!("Cloning {} from checkpoint {}...", new_agent_name, checkpoint_id);
        
        tracing::warn!("Note: Incus snapshot copy requires manual step: incus copy {}/{} {}",
            meta.agent_name, checkpoint_id, new_agent_name
        );

        current_state.agents.insert(new_agent_name.to_string(), new_agent);
        self.state_manager.save(&current_state).await?;

        tracing::info!("Agent {} created. Run 'incus copy {}/{} {}' to clone the snapshot.",
            new_agent_name, meta.agent_name, checkpoint_id, new_agent_name
        );
        Ok(())
    }

    pub async fn list_checkpoints(&self, agent_name: Option<&str>) -> Result<Vec<CheckpointMeta>> {
        self.state_manager.list_checkpoints(agent_name).await
    }

    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let (_, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        tracing::info!("Deleting snapshot {} from {}...", checkpoint_id, meta.agent_name);
        
        let op = self.incus_client.delete_snapshot(&meta.agent_name, checkpoint_id).await?;
        let result = self.incus_client.wait_operation(&op.id, 30).await?;
        if let Some(err) = result.err {
            if !err.is_empty() {
                return Err(Error::Config(format!("Failed to delete snapshot: {}", err)));
            }
        }

        self.state_manager.delete_checkpoint(checkpoint_id).await?;

        tracing::info!("Checkpoint {} deleted", checkpoint_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_checkpoints_constant() {
        assert_eq!(MAX_CHECKPOINTS_PER_AGENT, 10);
    }

    #[test]
    fn test_checkpoint_manager_paths() {
        let dir = tempfile::tempdir().unwrap();
        let manager = CheckpointManager::new(dir.path());
        assert!(manager.checkpoints_dir.ends_with("checkpoints"));
    }
}