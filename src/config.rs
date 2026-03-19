use crate::models::State;
use crate::Result;
use std::path::Path;

pub async fn export_to_yaml(state: &State, path: &Path) -> Result<()> {
    let content = serde_yaml::to_string(state)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

pub async fn import_from_yaml(path: &Path) -> Result<State> {
    let content = tokio::fs::read_to_string(path).await?;
    let state: State = serde_yaml::from_str(&content)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_export_to_yaml() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("config.yaml");
        
        let state = State::new();
        export_to_yaml(&state, &path).await.unwrap();
        
        assert!(path.exists());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("version"));
    }

    #[tokio::test]
    async fn test_import_from_yaml() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("config.yaml");
        
        let yaml = r#"version: "1.0"
agents: {}
providers: {}
api_keys: {}
forgejo_users: {}
skills: {}
tools: {}
"#;
        tokio::fs::write(&path, yaml).await.unwrap();
        
        let state = import_from_yaml(&path).await.unwrap();
        assert_eq!(state.version, "1.0");
    }

    #[tokio::test]
    async fn test_export_import_roundtrip() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("config.yaml");
        
        let original = State::new();
        export_to_yaml(&original, &path).await.unwrap();
        let loaded = import_from_yaml(&path).await.unwrap();
        
        assert_eq!(original.version, loaded.version);
    }
}