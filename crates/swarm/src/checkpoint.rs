use crate::btrfs::BtrfsManager;
use crate::models::{CheckpointMeta, State};
use crate::state::StateManager;
use crate::{Error, Result};
use std::path::Path;

const MAX_CHECKPOINTS_PER_AGENT: usize = 10;

pub struct CheckpointManager {
    state_manager: StateManager,
    btrfs_manager: BtrfsManager,
    checkpoints_dir: std::path::PathBuf,
}

impl CheckpointManager {
    pub fn new(data_dir: &Path, btrfs_device: &str, btrfs_mount: &Path) -> Self {
        Self {
            state_manager: StateManager::new(data_dir),
            btrfs_manager: BtrfsManager::new(btrfs_device, btrfs_mount),
            checkpoints_dir: data_dir.join("checkpoints"),
        }
    }

    pub async fn create_checkpoint(&self, agent_name: &str, description: &str) -> Result<String> {
        // Check if agent exists
        let state = self.state_manager.load().await?;
        if !state.agents.contains_key(agent_name) {
            return Err(Error::AgentNotFound(agent_name.to_string()));
        }

        // Check checkpoint limit
        let count = self.state_manager.count_checkpoints(agent_name).await?;
        if count >= MAX_CHECKPOINTS_PER_AGENT {
            return Err(Error::Validation(format!(
                "Agent {} already has {} checkpoints (max {}). Please delete old checkpoints first.",
                agent_name, count, MAX_CHECKPOINTS_PER_AGENT
            )));
        }

        // Generate checkpoint ID
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let short_uuid = &uuid::Uuid::new_v4().to_string()[..8];
        let checkpoint_id = format!("cp_{}_{}", timestamp, short_uuid);

        // Create checkpoint directory
        let checkpoint_dir = self.checkpoints_dir.join(&checkpoint_id);
        tokio::fs::create_dir_all(&checkpoint_dir).await?;

        // Save state snapshot
        let state_path = checkpoint_dir.join("state.json");
        let state_content = serde_json::to_string_pretty(&state)?;
        tokio::fs::write(&state_path, state_content).await?;

        // Create btrfs snapshot
        self.btrfs_manager.create_snapshot(agent_name, &checkpoint_id).await?;

        // Save meta
        let meta = CheckpointMeta {
            id: checkpoint_id.clone(),
            agent_name: agent_name.to_string(),
            description: description.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            btrfs_snapshot: self.btrfs_manager.snapshot_subvol_path(&checkpoint_id),
        };

        let meta_path = checkpoint_dir.join("meta.json");
        let meta_content = serde_json::to_string_pretty(&meta)?;
        tokio::fs::write(&meta_path, meta_content).await?;

        Ok(checkpoint_id)
    }

    pub async fn rollback(&self, agent_name: &str, checkpoint_id: &str) -> Result<()> {
        // Load checkpoint
        let (checkpoint_state, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        // Verify agent name matches
        if meta.agent_name != agent_name {
            return Err(Error::Validation(format!(
                "Checkpoint {} is for agent {}, not {}",
                checkpoint_id, meta.agent_name, agent_name
            )));
        }

        // Get current state
        let mut current_state = self.state_manager.load().await?;

        // Stop container
        tracing::info!("Stopping container {}...", agent_name);
        let _ = tokio::process::Command::new("systemctl")
            .args(["stop", &format!("container@{}", agent_name)])
            .status()
            .await;

        // Restore btrfs snapshot
        tracing::info!("Restoring filesystem snapshot...");
        self.btrfs_manager.restore_snapshot(checkpoint_id, agent_name).await?;

        // Restore agent config from checkpoint
        if let Some(agent_config) = checkpoint_state.agents.get(agent_name) {
            current_state.agents.insert(agent_name.to_string(), agent_config.clone());
        }

        // Save restored state
        self.state_manager.save(&current_state).await?;

        tracing::info!("Rollback complete. Please run 'zerg-swarm apply' to rebuild the system.");
        Ok(())
    }

    pub async fn clone(&self, checkpoint_id: &str, new_agent_name: &str) -> Result<()> {
        // Load checkpoint
        let (checkpoint_state, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        // Check if new agent name already exists
        let mut current_state = self.state_manager.load().await?;
        if current_state.agents.contains_key(new_agent_name) {
            return Err(Error::AgentAlreadyExists(new_agent_name.to_string()));
        }

        // Get source agent config
        let source_agent = checkpoint_state.agents.get(&meta.agent_name)
            .ok_or_else(|| Error::AgentNotFound(meta.agent_name.clone()))?;

        // Create new agent config
        let now = chrono::Utc::now().to_rfc3339();
        let agent_num = current_state.agents.len() + 1;
        let defaults = &current_state.defaults;

        let new_agent = crate::models::Agent {
            enabled: true,
            container_ip: format!("{}.{}.2", defaults.container_subnet_base, agent_num),
            host_ip: format!("{}.{}.1", defaults.container_subnet_base, agent_num),
            forgejo_username: Some(new_agent_name.to_string()),
            internal_token: uuid::Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        // Clone btrfs snapshot
        tracing::info!("Cloning filesystem...");
        self.btrfs_manager.clone_snapshot_to_agent(checkpoint_id, new_agent_name).await?;

        // Add to state
        current_state.agents.insert(new_agent_name.to_string(), new_agent);
        self.state_manager.save(&current_state).await?;

        tracing::info!("Agent {} cloned from checkpoint {}. Please run 'zerg-swarm apply' to create the container.", new_agent_name, checkpoint_id);
        Ok(())
    }

    pub async fn list_checkpoints(&self, agent_name: Option<&str>) -> Result<Vec<CheckpointMeta>> {
        self.state_manager.list_checkpoints(agent_name).await
    }

    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        // Load meta to get snapshot path
        let (_, meta) = self.state_manager.load_checkpoint(checkpoint_id).await?;

        // Delete btrfs snapshot
        self.btrfs_manager.delete_snapshot(checkpoint_id).await?;

        // Delete checkpoint directory
        self.state_manager.delete_checkpoint(checkpoint_id).await?;

        tracing::info!("Checkpoint {} deleted", checkpoint_id);
        Ok(())
    }
}