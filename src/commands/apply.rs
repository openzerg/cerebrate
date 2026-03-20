use cerebrate::Result;
use crate::state_init::init_state;

pub async fn handle_apply(
    data_dir: std::path::PathBuf,
    template: Option<std::path::PathBuf>,
) -> Result<()> {
    let state = init_state(data_dir.clone()).await?;
    let sw = state.state_manager.load().await?;
    
    let template_dir = template.unwrap_or_else(|| data_dir.join("system"));
    let manager = state.agent_manager.clone();
    
    println!("Applying Incus container configuration...");
    manager.apply(&sw).await?;
    println!("Incus containers configured successfully!");
    
    Ok(())
}