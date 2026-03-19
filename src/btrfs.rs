use crate::{Error, Result};
use std::path::Path;

const AGENTS_SUBVOL: &str = "@agents";
const SNAPSHOTS_SUBVOL: &str = "@snapshots";

pub struct BtrfsManager {
    btrfs_device: String,
    mount_point: std::path::PathBuf,
}

impl BtrfsManager {
    pub fn new(btrfs_device: &str, mount_point: &Path) -> Self {
        Self {
            btrfs_device: btrfs_device.to_string(),
            mount_point: mount_point.to_path_buf(),
        }
    }

    pub fn agent_subvol_path(&self, agent_name: &str) -> String {
        format!("{}/{}", AGENTS_SUBVOL, agent_name)
    }

    pub fn snapshot_subvol_path(&self, checkpoint_id: &str) -> String {
        format!("{}/{}", SNAPSHOTS_SUBVOL, checkpoint_id)
    }

    pub fn agent_mount_path(&self, agent_name: &str) -> std::path::PathBuf {
        self.mount_point.join(AGENTS_SUBVOL).join(agent_name)
    }

    pub async fn create_agent_subvolume(&self, agent_name: &str) -> Result<()> {
        let subvol_path = self.agent_subvol_path(agent_name);
        
        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "create", &subvol_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            return Err(Error::Config(format!("Failed to create btrfs subvolume for agent {}", agent_name)));
        }

        Ok(())
    }

    pub async fn delete_agent_subvolume(&self, agent_name: &str) -> Result<()> {
        let subvol_path = self.agent_subvol_path(agent_name);
        
        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "delete", &subvol_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            tracing::warn!("Failed to delete btrfs subvolume for agent {}", agent_name);
        }

        Ok(())
    }

    pub async fn create_snapshot(&self, agent_name: &str, checkpoint_id: &str) -> Result<()> {
        let source_path = self.agent_subvol_path(agent_name);
        let snapshot_path = self.snapshot_subvol_path(checkpoint_id);

        // Ensure snapshots directory exists
        let snapshots_dir = self.mount_point.join(SNAPSHOTS_SUBVOL);
        if !snapshots_dir.exists() {
            let status = tokio::process::Command::new("btrfs")
                .args(["subvolume", "create", SNAPSHOTS_SUBVOL])
                .current_dir(&self.mount_point)
                .status()
                .await
                .map_err(|e| Error::Io(e))?;

            if !status.success() {
                return Err(Error::Config("Failed to create snapshots subvolume".to_string()));
            }
        }

        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "snapshot", &source_path, &snapshot_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            return Err(Error::Config(format!("Failed to create snapshot {}", checkpoint_id)));
        }

        Ok(())
    }

    pub async fn restore_snapshot(&self, checkpoint_id: &str, agent_name: &str) -> Result<()> {
        let snapshot_path = self.snapshot_subvol_path(checkpoint_id);
        let target_path = self.agent_subvol_path(agent_name);

        // Delete current agent subvolume
        self.delete_agent_subvolume(agent_name).await?;

        // Create new subvolume from snapshot
        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "snapshot", &snapshot_path, &target_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            return Err(Error::Config(format!("Failed to restore snapshot {} to agent {}", checkpoint_id, agent_name)));
        }

        Ok(())
    }

    pub async fn delete_snapshot(&self, checkpoint_id: &str) -> Result<()> {
        let snapshot_path = self.snapshot_subvol_path(checkpoint_id);
        
        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "delete", &snapshot_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            tracing::warn!("Failed to delete snapshot {}", checkpoint_id);
        }

        Ok(())
    }

    pub async fn clone_snapshot_to_agent(&self, checkpoint_id: &str, new_agent_name: &str) -> Result<()> {
        let snapshot_path = self.snapshot_subvol_path(checkpoint_id);
        let target_path = self.agent_subvol_path(new_agent_name);

        let status = tokio::process::Command::new("btrfs")
            .args(["subvolume", "snapshot", &snapshot_path, &target_path])
            .current_dir(&self.mount_point)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;

        if !status.success() {
            return Err(Error::Config(format!("Failed to clone snapshot to agent {}", new_agent_name)));
        }

        Ok(())
    }

    pub async fn ensure_agents_subvolume(&self) -> Result<()> {
        let agents_dir = self.mount_point.join(AGENTS_SUBVOL);
        if !agents_dir.exists() {
            let status = tokio::process::Command::new("btrfs")
                .args(["subvolume", "create", AGENTS_SUBVOL])
                .current_dir(&self.mount_point)
                .status()
                .await
                .map_err(|e| Error::Io(e))?;

            if !status.success() {
                return Err(Error::Config("Failed to create agents subvolume".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_btrfs_manager_new() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.btrfs_device, "/dev/sda1");
        assert_eq!(manager.mount_point, dir.path());
    }

    #[test]
    fn test_agent_subvol_path() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.agent_subvol_path("my-agent"), "@agents/my-agent");
    }

    #[test]
    fn test_snapshot_subvol_path() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.snapshot_subvol_path("cp-123"), "@snapshots/cp-123");
    }

    #[test]
    fn test_agent_mount_path() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        let path = manager.agent_mount_path("test-agent");
        assert_eq!(path, dir.path().join("@agents").join("test-agent"));
    }

    #[test]
    fn test_agent_subvol_path_special_chars() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.agent_subvol_path("agent-123_test"), "@agents/agent-123_test");
    }

    #[test]
    fn test_snapshot_subvol_path_with_uuid() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        let path = manager.snapshot_subvol_path("cp_20240101_abc123");
        assert_eq!(path, "@snapshots/cp_20240101_abc123");
    }

    #[test]
    fn test_agent_mount_path_nested() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/nvme0n1", dir.path());
        let path = manager.agent_mount_path("nested-agent");
        assert!(path.to_str().unwrap().contains("@agents"));
        assert!(path.to_str().unwrap().contains("nested-agent"));
    }

    #[test]
    fn test_multiple_managers() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let manager1 = BtrfsManager::new("/dev/sda1", dir1.path());
        let manager2 = BtrfsManager::new("/dev/sda2", dir2.path());
        assert_ne!(manager1.mount_point, manager2.mount_point);
        assert_ne!(manager1.btrfs_device, manager2.btrfs_device);
    }

    #[test]
    fn test_agent_subvol_path_multiple() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.agent_subvol_path("agent1"), "@agents/agent1");
        assert_eq!(manager.agent_subvol_path("agent2"), "@agents/agent2");
        assert_eq!(manager.agent_subvol_path("my-agent"), "@agents/my-agent");
    }

    #[test]
    fn test_snapshot_subvol_path_multiple() {
        let dir = tempdir().unwrap();
        let manager = BtrfsManager::new("/dev/sda1", dir.path());
        assert_eq!(manager.snapshot_subvol_path("cp1"), "@snapshots/cp1");
        assert_eq!(manager.snapshot_subvol_path("cp2"), "@snapshots/cp2");
    }

    #[test]
    fn test_agent_mount_path_different_devices() {
        let dir = tempdir().unwrap();
        let manager1 = BtrfsManager::new("/dev/sda1", dir.path());
        let manager2 = BtrfsManager::new("/dev/nvme0n1", dir.path());
        
        assert_eq!(manager1.agent_mount_path("agent"), manager2.agent_mount_path("agent"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(AGENTS_SUBVOL, "@agents");
        assert_eq!(SNAPSHOTS_SUBVOL, "@snapshots");
    }
}