use std::collections::HashMap;
use swarm::{Result, ApiKey};
use swarm::state;
use crate::cli::KeyCommands;

pub async fn handle_key_command(command: KeyCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    
    match command {
        KeyCommands::List => {
            let keys = &sw.api_keys;
            let providers = &sw.providers;
            let provider_map: HashMap<_, _> = providers.iter().map(|(k, v)| (k.clone(), v.name.clone())).collect();
            let unknown = "unknown".to_string();
            
            if keys.is_empty() {
                println!("No API keys found.");
            } else {
                println!("{:<36} {:<20} {:<20}", "ID", "NAME", "PROVIDER");
                println!("{}", "-".repeat(80));
                for (id, k) in keys {
                    let provider_name = provider_map.get(&k.provider_id).unwrap_or(&unknown);
                    println!("{:<36} {:<20} {:<20}", id, k.name, provider_name);
                }
            }
        }
        
        KeyCommands::Create { name, provider } => {
            let id = uuid::Uuid::new_v4().to_string();
            let raw_key = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(raw_key.as_bytes());
            let key_hash = format!("{:x}", hasher.finalize());
            
            let api_key = ApiKey {
                id: id.clone(),
                name,
                key_hash,
                provider_id: provider,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.api_keys.insert(id.clone(), api_key.clone());
            state_manager.save(&sw).await?;
            
            println!("API key '{}' created", api_key.name);
            println!("  ID: {}", api_key.id);
            println!("  Raw key (save this, it won't be shown again): {}", raw_key);
        }
        
        KeyCommands::Delete { id } => {
            if sw.api_keys.remove(&id).is_none() {
                eprintln!("API key '{}' not found", id);
                std::process::exit(1);
            }
            state_manager.save(&sw).await?;
            println!("API key '{}' deleted", id);
        }
    }
    
    Ok(())
}