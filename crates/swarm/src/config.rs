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