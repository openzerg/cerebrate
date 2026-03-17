use crate::models::State;
use crate::{Error, Result};
use std::path::Path;

pub struct StateManager {
    path: std::path::PathBuf,
}

impl StateManager {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("state.json"),
        }
    }

    pub async fn load(&self) -> Result<State> {
        if !self.path.exists() {
            return Ok(State::new());
        }

        let content = tokio::fs::read_to_string(&self.path).await?;
        let state: State = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub async fn save(&self, state: &State) -> Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }

    pub async fn load_checkpoint(&self, checkpoint_id: &str) -> Result<(State, crate::models::CheckpointMeta)> {
        let checkpoint_dir = self.path.parent()
            .ok_or_else(|| Error::Config("Invalid data directory".to_string()))?
            .join("checkpoints")
            .join(checkpoint_id);

        if !checkpoint_dir.exists() {
            return Err(Error::NotFound(format!("Checkpoint {} not found", checkpoint_id)));
        }

        let state_path = checkpoint_dir.join("state.json");
        let meta_path = checkpoint_dir.join("meta.json");

        let state_content = tokio::fs::read_to_string(&state_path).await?;
        let meta_content = tokio::fs::read_to_string(&meta_path).await?;

        let state: State = serde_json::from_str(&state_content)?;
        let meta: crate::models::CheckpointMeta = serde_json::from_str(&meta_content)?;

        Ok((state, meta))
    }

    pub async fn list_checkpoints(&self, agent_name: Option<&str>) -> Result<Vec<crate::models::CheckpointMeta>> {
        let checkpoints_dir = self.path.parent()
            .ok_or_else(|| Error::Config("Invalid data directory".to_string()))?
            .join("checkpoints");

        if !checkpoints_dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let mut entries = tokio::fs::read_dir(&checkpoints_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let meta_path = entry.path().join("meta.json");
            if meta_path.exists() {
                let meta_content = tokio::fs::read_to_string(&meta_path).await?;
                let meta: crate::models::CheckpointMeta = serde_json::from_str(&meta_content)?;
                
                if let Some(name) = agent_name {
                    if meta.agent_name != name {
                        continue;
                    }
                }
                checkpoints.push(meta);
            }
        }

        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(checkpoints)
    }

    pub async fn count_checkpoints(&self, agent_name: &str) -> Result<usize> {
        let checkpoints = self.list_checkpoints(Some(agent_name)).await?;
        Ok(checkpoints.len())
    }

    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let checkpoint_dir = self.path.parent()
            .ok_or_else(|| Error::Config("Invalid data directory".to_string()))?
            .join("checkpoints")
            .join(checkpoint_id);

        if checkpoint_dir.exists() {
            tokio::fs::remove_dir_all(&checkpoint_dir).await?;
        }
        Ok(())
    }
}