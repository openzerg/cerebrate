use crate::error::{Error, Result};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct GitOps {
    data_dir: PathBuf,
    forgejo_url: String,
    forgejo_token: String,
}

impl GitOps {
    pub fn new(data_dir: PathBuf, forgejo_url: String, forgejo_token: String) -> Self {
        Self { data_dir, forgejo_url, forgejo_token }
    }

    pub fn tool_dir(&self, slug: &str) -> PathBuf {
        self.data_dir.join("tools").join(slug)
    }

    pub fn skill_dir(&self, slug: &str) -> PathBuf {
        self.data_dir.join("skills").join(slug)
    }

    pub async fn clone(&self, target_dir: &PathBuf, forgejo_repo: &str) -> Result<()> {
        if target_dir.exists() {
            fs::remove_dir_all(target_dir).await?;
        }
        
        let repo_url = format!("{}/{}.git", self.forgejo_url.trim_end_matches('/'), forgejo_repo);
        
        let status = tokio::process::Command::new("git")
            .args(["clone", "--depth", "1", &repo_url, &target_dir.display().to_string()])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "true")
            .env("GIT_USERNAME", "oauth2")
            .env("GIT_PASSWORD", &self.forgejo_token)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;
        
        if !status.success() {
            return Err(Error::TaskFailed(format!("Failed to clone from {}", repo_url)));
        }
        
        Ok(())
    }

    pub async fn pull(&self, target_dir: &PathBuf) -> Result<String> {
        if !target_dir.exists() {
            return Err(Error::NotFound("Directory not found".into()));
        }
        
        let output = tokio::process::Command::new("git")
            .args(["pull", "--force"])
            .current_dir(target_dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "true")
            .env("GIT_USERNAME", "oauth2")
            .env("GIT_PASSWORD", &self.forgejo_token)
            .output()
            .await
            .map_err(|e| Error::Io(e))?;
        
        if !output.status.success() {
            return Err(Error::TaskFailed(format!("Failed to pull: {}", 
                String::from_utf8_lossy(&output.stderr))));
        }
        
        self.get_commit(target_dir).await
    }

    pub async fn get_commit(&self, target_dir: &PathBuf) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(target_dir)
            .output()
            .await
            .map_err(|e| Error::Io(e))?;
        
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn delete(&self, target_dir: &PathBuf) -> Result<()> {
        if target_dir.exists() {
            fs::remove_dir_all(target_dir).await?;
        }
        Ok(())
    }
}