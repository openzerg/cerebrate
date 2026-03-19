use swarm::{Result, Agent};
use swarm::state;
use swarm::checkpoint;
use crate::cli::AgentCommands;

pub async fn handle_agent_command(command: AgentCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    
    match command {
        AgentCommands::List => {
            let agents = &sw.agents;
            if agents.is_empty() {
                println!("No agents found.");
            } else {
                println!("{:<20} {:<8} {:<15} {:<15}", "NAME", "ENABLED", "CONTAINER_IP", "HOST_IP");
                println!("{}", "-".repeat(60));
                for (name, agent) in agents {
                    println!("{:<20} {:<8} {:<15} {:<15}", 
                        name, 
                        if agent.enabled { "yes" } else { "no" },
                        agent.container_ip,
                        agent.host_ip
                    );
                }
            }
        }
        
        AgentCommands::Create { name, forgejo_username } => {
            if sw.agents.contains_key(&name) {
                eprintln!("Error: Agent '{}' already exists", name);
                std::process::exit(1);
            }
            
            let agent_num = sw.agents.len() + 1;
            let now = chrono::Utc::now().to_rfc3339();
            
            let agent = Agent {
                enabled: true,
                container_ip: format!("{}.{}.2", sw.defaults.container_subnet_base, agent_num),
                host_ip: format!("{}.{}.1", sw.defaults.container_subnet_base, agent_num),
                forgejo_username: forgejo_username.or(Some(name.clone())),
                internal_token: uuid::Uuid::new_v4().to_string(),
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.agents.insert(name.clone(), agent.clone());
            state_manager.save(&sw).await?;
            
            println!("Agent '{}' created:", name);
            println!("  Container IP: {}", agent.container_ip);
            println!("  Host IP: {}", agent.host_ip);
            println!("  Internal Token: {}", agent.internal_token);
        }
        
        AgentCommands::Get { name } => {
            match sw.agents.get(&name) {
                Some(agent) => {
                    println!("Agent: {}", name);
                    println!("  Enabled: {}", agent.enabled);
                    println!("  Container IP: {}", agent.container_ip);
                    println!("  Host IP: {}", agent.host_ip);
                    println!("  Forgejo Username: {:?}", agent.forgejo_username);
                    println!("  Internal Token: {}", agent.internal_token);
                    println!("  Created: {}", agent.created_at);
                    println!("  Updated: {}", agent.updated_at);
                }
                None => {
                    eprintln!("Agent '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }
        
        AgentCommands::Delete { name } => {
            if sw.agents.remove(&name).is_none() {
                eprintln!("Agent '{}' not found", name);
                std::process::exit(1);
            }
            state_manager.save(&sw).await?;
            println!("Agent '{}' deleted", name);
        }
        
        AgentCommands::Enable { name } => {
            if let Some(agent) = sw.agents.get_mut(&name) {
                agent.enabled = true;
                agent.updated_at = chrono::Utc::now().to_rfc3339();
                state_manager.save(&sw).await?;
                println!("Agent '{}' enabled", name);
            } else {
                eprintln!("Agent '{}' not found", name);
                std::process::exit(1);
            }
        }
        
        AgentCommands::Disable { name } => {
            if let Some(agent) = sw.agents.get_mut(&name) {
                agent.enabled = false;
                agent.updated_at = chrono::Utc::now().to_rfc3339();
                state_manager.save(&sw).await?;
                println!("Agent '{}' disabled", name);
            } else {
                eprintln!("Agent '{}' not found", name);
                std::process::exit(1);
            }
        }
        
        AgentCommands::Checkpoint { name, desc } => {
            let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
            let checkpoint_id = checkpoint_mgr.create_checkpoint(&name, desc.as_deref().unwrap_or("")).await?;
            println!("Checkpoint '{}' created for agent '{}'", checkpoint_id, name);
        }
        
        AgentCommands::Rollback { name, checkpoint_id } => {
            let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
            checkpoint_mgr.rollback(&name, &checkpoint_id).await?;
            println!("Rolled back agent '{}' to checkpoint '{}'", name, checkpoint_id);
        }
        
        AgentCommands::ListCheckpoints { name } => {
            let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
            let checkpoints = checkpoint_mgr.list_checkpoints(Some(&name)).await?;
            
            if checkpoints.is_empty() {
                println!("No checkpoints found for agent '{}'", name);
            } else {
                println!("Checkpoints for agent '{}':\n", name);
                println!("{:<30} {:<20} {}", "ID", "CREATED", "DESCRIPTION");
                println!("{}", "-".repeat(70));
                for cp in checkpoints {
                    println!("{:<30} {:<20} {}", cp.id, cp.created_at, cp.description);
                }
            }
        }
        
        AgentCommands::DeleteCheckpoint { checkpoint_id } => {
            let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
            checkpoint_mgr.delete_checkpoint(&checkpoint_id).await?;
            println!("Checkpoint '{}' deleted", checkpoint_id);
        }
    }
    
    Ok(())
}