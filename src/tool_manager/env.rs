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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_env_dir_path() {
        let data_dir = PathBuf::from("/data");
        let result = env_dir(&data_dir, "my-tool");
        assert_eq!(result, PathBuf::from("/data/tools/my-tool/env"));
    }

    #[tokio::test]
    async fn test_set_and_list() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        set(&data_dir, "test-tool", "API_KEY", "secret123").await.unwrap();
        set(&data_dir, "test-tool", "OTHER_VAR", "value").await.unwrap();
        
        let keys = list(&data_dir, "test-tool").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"API_KEY".to_string()));
        assert!(keys.contains(&"OTHER_VAR".to_string()));
    }

    #[tokio::test]
    async fn test_set_overwrites() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        set(&data_dir, "tool", "KEY", "value1").await.unwrap();
        set(&data_dir, "tool", "KEY", "value2").await.unwrap();
        
        let keys = list(&data_dir, "tool").await.unwrap();
        assert_eq!(keys.len(), 1);
    }

    #[tokio::test]
    async fn test_list_nonexistent_tool() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        let keys = list(&data_dir, "nonexistent").await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        set(&data_dir, "tool", "KEY1", "val1").await.unwrap();
        set(&data_dir, "tool", "KEY2", "val2").await.unwrap();
        
        delete(&data_dir, "tool", "KEY1").await.unwrap();
        
        let keys = list(&data_dir, "tool").await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"KEY2".to_string()));
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        let result = delete(&data_dir, "tool", "NONEXISTENT").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_all() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        std::fs::create_dir_all(data_dir.join("tools").join("mytool").join("env")).unwrap();
        std::fs::write(data_dir.join("tools").join("mytool").join("env").join("KEY1"), "value1").unwrap();
        std::fs::write(data_dir.join("tools").join("mytool").join("env").join("KEY2"), "value2").unwrap();
        
        let env = load_all(&data_dir, "mytool");
        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_load_all_nonexistent() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        
        let env = load_all(&data_dir, "nonexistent");
        assert!(env.is_empty());
    }

    #[test]
    fn test_load_all_ignores_directories() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        let env_path = data_dir.join("tools").join("tool").join("env");
        
        std::fs::create_dir_all(&env_path).unwrap();
        std::fs::write(env_path.join("KEY"), "value").unwrap();
        std::fs::create_dir(env_path.join("SUBDIR")).unwrap();
        
        let env = load_all(&data_dir, "tool");
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("KEY"), Some(&"value".to_string()));
    }
}