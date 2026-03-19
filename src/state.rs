use crate::models::State;
use crate::{Error, Result};
use std::path::Path;

#[derive(Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_manager_new() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        assert!(manager.path.ends_with("state.json"));
    }

    #[tokio::test]
    async fn test_load_empty() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        let state = manager.load().await.unwrap();
        assert_eq!(state.version, "1.0");
        assert!(state.agents.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let mut state = State::new();
        state.agents.insert("agent-1".to_string(), crate::models::Agent {
            enabled: true,
            container_ip: "10.200.1.2".to_string(),
            host_ip: "10.200.1.1".to_string(),
            forgejo_username: None,
            internal_token: "token".to_string(),
            model_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        });
        
        manager.save(&state).await.unwrap();
        let loaded = manager.load().await.unwrap();
        
        assert_eq!(loaded.agents.len(), 1);
        assert!(loaded.agents.contains_key("agent-1"));
    }

    #[tokio::test]
    async fn test_list_checkpoints_empty() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let checkpoints = manager.list_checkpoints(None).await.unwrap();
        assert!(checkpoints.is_empty());
    }

    #[tokio::test]
    async fn test_count_checkpoints() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let count = manager.count_checkpoints("agent-1").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_checkpoint_nonexistent() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let result = manager.delete_checkpoint("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_checkpoint_not_found() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let result = manager.load_checkpoint("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_checkpoints_with_checkpoint() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let checkpoints_dir = temp.path().join("checkpoints").join("cp_test");
        tokio::fs::create_dir_all(&checkpoints_dir).await.unwrap();
        
        let meta = crate::models::CheckpointMeta {
            id: "cp_test".to_string(),
            agent_name: "agent-1".to_string(),
            description: "Test checkpoint".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            btrfs_snapshot: "@snapshots/cp_test".to_string(),
        };
        
        let meta_content = serde_json::to_string_pretty(&meta).unwrap();
        tokio::fs::write(checkpoints_dir.join("meta.json"), meta_content).await.unwrap();
        
        let state = State::new();
        let state_content = serde_json::to_string_pretty(&state).unwrap();
        tokio::fs::write(checkpoints_dir.join("state.json"), state_content).await.unwrap();
        
        let checkpoints = manager.list_checkpoints(None).await.unwrap();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].id, "cp_test");
    }

    #[tokio::test]
    async fn test_list_checkpoints_filter_by_agent() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        for i in 1..=2 {
            let checkpoints_dir = temp.path().join("checkpoints").join(format!("cp_{}", i));
            tokio::fs::create_dir_all(&checkpoints_dir).await.unwrap();
            
            let meta = crate::models::CheckpointMeta {
                id: format!("cp_{}", i),
                agent_name: format!("agent-{}", i),
                description: "Test".to_string(),
                created_at: format!("2024-01-0{}T00:00:00Z", i),
                btrfs_snapshot: format!("@snapshots/cp_{}", i),
            };
            
            let meta_content = serde_json::to_string_pretty(&meta).unwrap();
            tokio::fs::write(checkpoints_dir.join("meta.json"), meta_content).await.unwrap();
            
            let state = State::new();
            let state_content = serde_json::to_string_pretty(&state).unwrap();
            tokio::fs::write(checkpoints_dir.join("state.json"), state_content).await.unwrap();
        }
        
        let checkpoints = manager.list_checkpoints(Some("agent-1")).await.unwrap();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].agent_name, "agent-1");
    }

    #[tokio::test]
    async fn test_load_checkpoint_success() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let checkpoints_dir = temp.path().join("checkpoints").join("cp_success");
        tokio::fs::create_dir_all(&checkpoints_dir).await.unwrap();
        
        let meta = crate::models::CheckpointMeta {
            id: "cp_success".to_string(),
            agent_name: "agent-1".to_string(),
            description: "Success test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            btrfs_snapshot: "@snapshots/cp_success".to_string(),
        };
        
        tokio::fs::write(checkpoints_dir.join("meta.json"), serde_json::to_string_pretty(&meta).unwrap()).await.unwrap();
        tokio::fs::write(checkpoints_dir.join("state.json"), serde_json::to_string_pretty(&State::new()).unwrap()).await.unwrap();
        
        let (loaded_state, loaded_meta) = manager.load_checkpoint("cp_success").await.unwrap();
        assert_eq!(loaded_meta.id, "cp_success");
        assert_eq!(loaded_state.version, "1.0");
    }

    #[tokio::test]
    async fn test_delete_checkpoint_existing() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        
        let checkpoints_dir = temp.path().join("checkpoints").join("cp_delete");
        tokio::fs::create_dir_all(&checkpoints_dir).await.unwrap();
        tokio::fs::write(checkpoints_dir.join("meta.json"), "{}").await.unwrap();
        
        assert!(checkpoints_dir.exists());
        manager.delete_checkpoint("cp_delete").await.unwrap();
        assert!(!checkpoints_dir.exists());
    }

    #[test]
    fn test_state_manager_clone() {
        let temp = tempdir().unwrap();
        let manager = StateManager::new(temp.path());
        let cloned = manager.clone();
        assert_eq!(manager.path, cloned.path);
    }
}