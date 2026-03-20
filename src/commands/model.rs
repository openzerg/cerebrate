use std::collections::HashMap;
use cerebrate::Result;
use cerebrate::state;
use crate::cli::ModelCommands;

pub async fn handle_model_command(command: ModelCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    
    match command {
        ModelCommands::List => {
            let models = &sw.models;
            let providers = &sw.providers;
            let provider_map: HashMap<_, _> = providers.iter().map(|(k, v)| (k.clone(), v.name.clone())).collect();
            let unknown = "unknown".to_string();
            
            if models.is_empty() {
                println!("No models found.");
            } else {
                println!("{:<36} {:<20} {:<20} {:<20}", "ID", "NAME", "MODEL", "PROVIDER");
                println!("{}", "-".repeat(100));
                for (id, m) in models {
                    let provider_name = provider_map.get(&m.provider_id).unwrap_or(&unknown);
                    println!("{:<36} {:<20} {:<20} {:<20}", id, m.name, m.model_name, provider_name);
                }
            }
        }
        
        ModelCommands::Create { name, provider, model_name } => {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            
            let model = cerebrate::Model {
                id: id.clone(),
                name,
                provider_id: provider,
                model_name,
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.models.insert(id.clone(), model.clone());
            state_manager.save(&sw).await?;
            
            println!("Model '{}' created", model.name);
            println!("  ID: {}", model.id);
            println!("  Model: {}", model.model_name);
        }
        
        ModelCommands::Delete { id } => {
            if sw.models.remove(&id).is_none() {
                eprintln!("Model '{}' not found", id);
                std::process::exit(1);
            }
            
            for agent in sw.agents.values_mut() {
                if agent.model_id.as_ref() == Some(&id) {
                    agent.model_id = None;
                }
            }
            
            state_manager.save(&sw).await?;
            println!("Model '{}' deleted", id);
        }
    }
    
    Ok(())
}