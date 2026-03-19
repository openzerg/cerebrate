use swarm::Result;
use swarm::state;
use swarm::config;
use swarm::sync;
use crate::cli::ConfigCommands;

pub async fn handle_config_command(command: ConfigCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let sw = state_manager.load().await?;
    
    match command {
        ConfigCommands::Export => {
            let export_path = data_dir.join("config.yaml");
            config::export_to_yaml(&sw, &export_path).await?;
            println!("Config exported to {:?}", export_path);
        }
        ConfigCommands::Import => {
            let import_path = data_dir.join("config.yaml");
            let imported = config::import_from_yaml(&import_path).await?;
            state_manager.save(&imported).await?;
            println!("Config imported from {:?}", import_path);
            
            println!("\nSyncing state to Forgejo and local tools/skills...");
            let result = sync::sync_all(&imported, &data_dir, false).await?;
            result.print_summary();
        }
        ConfigCommands::Sync { delete } => {
            let result = sync::sync_all(&sw, &data_dir, delete).await?;
            result.print_summary();
        }
    }
    Ok(())
}