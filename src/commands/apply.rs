use swarm::Result;
use crate::state_init::init_state;

pub async fn handle_apply(
    data_dir: std::path::PathBuf,
    template: Option<std::path::PathBuf>,
    btrfs_device: &str,
) -> Result<()> {
    let state = init_state(data_dir.clone()).await?;
    let sw = state.state_manager.load().await?;
    
    let template_dir = template.unwrap_or_else(|| data_dir.join("system"));
    let manager = state.agent_manager.clone().with_template(&template_dir);
    
    println!("Applying NixOS configuration...");
    manager.apply(&sw, btrfs_device).await?;
    println!("NixOS configuration applied successfully!");
    
    Ok(())
}