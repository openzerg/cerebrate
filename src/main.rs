mod cli;
mod commands;
mod state_init;

use clap::Parser;
use cli::{Cli, Commands, get_data_dir};
use swarm::Result;

#[tokio::main]
async fn main() -> swarm::Result<()> {
    let cli = Cli::parse();
    let data_dir = get_data_dir(cli.data_dir);
    
    match cli.command {
        Commands::Status => {
            commands::handle_status().await?;
        }
        
        Commands::Serve => {
            commands::handle_serve(data_dir).await?;
        }
        
        Commands::Apply { template, btrfs_device } => {
            commands::handle_apply(data_dir, template, &btrfs_device).await?;
        }
        
        Commands::Agent { command } => {
            commands::handle_agent_command(command, data_dir).await?;
        }
        
        Commands::Checkpoint { command } => {
            commands::handle_checkpoint_command(command, data_dir).await?;
        }
        
        Commands::Git { command } => {
            commands::handle_git_command(command, data_dir).await?;
        }
        
        Commands::Config { command } => {
            commands::handle_config_command(command, data_dir).await?;
        }
        
        Commands::Provider { command } => {
            commands::handle_provider_command(command, data_dir).await?;
        }
        
        Commands::Key { command } => {
            commands::handle_key_command(command, data_dir).await?;
        }
        
        Commands::Skill { command } => {
            commands::handle_skill_command(command, data_dir).await?;
        }
        
        Commands::Tool { command } => {
            commands::handle_tool_command(command, data_dir).await?;
        }
    }

    Ok(())
}