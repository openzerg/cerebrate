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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_git_ops_new() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://forgejo.example.com".to_string(),
            "token123".to_string(),
        );
        assert_eq!(git.data_dir, PathBuf::from("/data"));
        assert_eq!(git.forgejo_url, "https://forgejo.example.com");
        assert_eq!(git.forgejo_token, "token123");
    }

    #[test]
    fn test_tool_dir() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://forgejo.example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.tool_dir("my-tool"), PathBuf::from("/data/tools/my-tool"));
    }

    #[test]
    fn test_skill_dir() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://forgejo.example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.skill_dir("my-skill"), PathBuf::from("/data/skills/my-skill"));
    }

    #[test]
    fn test_tool_dir_nested_slug() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://forgejo.example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.tool_dir("org/tool"), PathBuf::from("/data/tools/org/tool"));
    }

    #[test]
    fn test_skill_dir_nested_slug() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://forgejo.example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.skill_dir("org/skill"), PathBuf::from("/data/skills/org/skill"));
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let git = GitOps::new(
            dir.path().to_path_buf(),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        let nonexistent = dir.path().join("nonexistent");
        let result = git.delete(&nonexistent).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_existing() {
        let dir = tempdir().unwrap();
        let git = GitOps::new(
            dir.path().to_path_buf(),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        let target = dir.path().join("to-delete");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("file.txt"), "content").unwrap();
        
        git.delete(&target).await.unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_git_ops_debug() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        let debug = format!("{:?}", git);
        assert!(debug.contains("GitOps"));
    }

    #[test]
    fn test_tool_dir_empty_slug() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.tool_dir(""), PathBuf::from("/data/tools"));
    }

    #[test]
    fn test_skill_dir_empty_slug() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.skill_dir(""), PathBuf::from("/data/skills"));
    }

    #[test]
    fn test_tool_dir_deeply_nested() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.tool_dir("org/team/project/tool"), PathBuf::from("/data/tools/org/team/project/tool"));
    }

    #[test]
    fn test_git_ops_with_trailing_url_slash() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com/".to_string(),
            "token".to_string(),
        );
        assert_eq!(git.forgejo_url, "https://example.com/");
    }

    #[test]
    fn test_git_ops_empty_token() {
        let git = GitOps::new(
            PathBuf::from("/data"),
            "https://example.com".to_string(),
            "".to_string(),
        );
        assert_eq!(git.forgejo_token, "");
    }
}