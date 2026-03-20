use cerebrate::{Result, Provider, ProviderType, Error};
use cerebrate::state;
use crate::cli::ProviderCommands;

pub async fn handle_provider_command(command: ProviderCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    
    match command {
        ProviderCommands::List => {
            let providers = &sw.providers;
            if providers.is_empty() {
                println!("No providers found.");
            } else {
                println!("{:<36} {:<15} {:<30}", "ID", "TYPE", "NAME");
                println!("{}", "-".repeat(85));
                for (id, p) in providers {
                    println!("{:<36} {:<15} {:<30}", id, p.provider_type.as_str(), p.name);
                }
            }
        }
        
        ProviderCommands::Create { name, provider_type, base_url, api_key } => {
            let pt = ProviderType::from_str(&provider_type)
                .ok_or_else(|| Error::Validation(format!("Invalid provider type: {}", provider_type)))?;
            
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            
            let provider = Provider {
                id: id.clone(),
                name,
                provider_type: pt,
                base_url,
                api_key,
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.providers.insert(id.clone(), provider.clone());
            state_manager.save(&sw).await?;
            
            println!("Provider '{}' created:", provider.name);
            println!("  ID: {}", provider.id);
            println!("  Type: {}", provider.provider_type.as_str());
        }
        
        ProviderCommands::Delete { id } => {
            if sw.providers.remove(&id).is_none() {
                eprintln!("Provider '{}' not found", id);
                std::process::exit(1);
            }
            state_manager.save(&sw).await?;
            println!("Provider '{}' deleted", id);
        }
    }
    
    Ok(())
}