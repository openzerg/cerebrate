use swarm::Result;
use swarm::checkpoint;
use crate::cli::CheckpointCommands;

pub async fn handle_checkpoint_command(command: CheckpointCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
    
    match command {
        CheckpointCommands::Clone { checkpoint_id, new_name } => {
            checkpoint_mgr.clone(&checkpoint_id, &new_name).await?;
            println!("Cloned checkpoint '{}' to new agent '{}'", checkpoint_id, new_name);
        }
        
        CheckpointCommands::List { agent } => {
            let checkpoints = checkpoint_mgr.list_checkpoints(agent.as_deref()).await?;
            
            if checkpoints.is_empty() {
                println!("No checkpoints found");
            } else {
                println!("{:<30} {:<15} {:<20} {}", "ID", "AGENT", "CREATED", "DESCRIPTION");
                println!("{}", "-".repeat(85));
                for cp in checkpoints {
                    println!("{:<30} {:<15} {:<20} {}", cp.id, cp.agent_name, cp.created_at, cp.description);
                }
            }
        }
    }
    
    Ok(())
}