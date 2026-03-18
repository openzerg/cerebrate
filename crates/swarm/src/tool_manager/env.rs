use crate::error::{Error, Result};
use std::path::PathBuf;
use tokio::fs;

fn env_dir(data_dir: &PathBuf, slug: &str) -> PathBuf {
    data_dir.join("tools").join(slug).join("env")
}

pub async fn set(data_dir: &PathBuf, slug: &str, key: &str, value: &str) -> Result<()> {
    let dir = env_dir(data_dir, slug);
    fs::create_dir_all(&dir).await?;
    let path = dir.join(key);
    fs::write(&path, value).await?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await?;
    }
    Ok(())
}

pub async fn list(data_dir: &PathBuf, slug: &str) -> Result<Vec<String>> {
    let dir = env_dir(data_dir, slug);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(&dir).await?;
    let mut keys = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                keys.push(name.to_string());
            }
        }
    }
    Ok(keys)
}

pub async fn delete(data_dir: &PathBuf, slug: &str, key: &str) -> Result<()> {
    let path = env_dir(data_dir, slug).join(key);
    if path.exists() {
        fs::remove_file(&path).await?;
    }
    Ok(())
}

pub fn load_all(data_dir: &PathBuf, slug: &str) -> std::collections::HashMap<String, String> {
    let mut env = std::collections::HashMap::new();
    let env_dir = env_dir(data_dir, slug);
    
    if let Ok(entries) = std::fs::read_dir(&env_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Some(key) = entry.file_name().to_str() {
                    if let Ok(value) = std::fs::read_to_string(entry.path()) {
                        env.insert(key.to_string(), value);
                    }
                }
            }
        }
    }
    env
}