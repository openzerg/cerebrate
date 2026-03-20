use cerebrate::{Result, Provider, ProviderType, Error};
use cerebrate::state;
use cerebrate::pylon_client::CreateProxyRequest;
use crate::cli::ProviderCommands;

pub async fn handle_provider_command(command: ProviderCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    let pylon_client = cerebrate::PylonClient::new();
    
    match command {
        ProviderCommands::List => {
            let providers = &sw.providers;
            if providers.is_empty() {
                println!("No providers found.");
            } else {
                println!("{:<36} {:<15} {:<30} {:<15}", "ID", "TYPE", "NAME", "PROXY");
                println!("{}", "-".repeat(100));
                for (id, p) in providers {
                    let proxy_status = p.pylon_proxy_id.as_ref().map(|_| "Yes").unwrap_or("No");
                    println!("{:<36} {:<15} {:<30} {:<15}", id, p.provider_type.as_str(), p.name, proxy_status);
                }
            }
        }
        
        ProviderCommands::Create { name, provider_type, base_url, api_key } => {
            let pt = ProviderType::from_str(&provider_type)
                .ok_or_else(|| Error::Validation(format!("Invalid provider type: {}", provider_type)))?;
            
            let id = uuid::Uuid::new_v4().to_string();
            let proxy_id = format!("{}-proxy", id);
            let source_model = name.to_lowercase().replace(' ', "-");
            
            let proxy_req = CreateProxyRequest {
                id: proxy_id.clone(),
                source_model: source_model.clone(),
                target_model: source_model.clone(),
                upstream: base_url.clone(),
                api_key,
                default_max_tokens: None,
                default_temperature: None,
                default_top_p: None,
                default_top_k: None,
                support_streaming: Some(true),
                support_tools: None,
                support_vision: None,
                extra_headers: None,
                extra_body: None,
            };
            
            pylon_client.create_proxy(&proxy_req).await
                .map_err(|e| Error::Config(format!("Failed to create Pylon proxy: {}", e)))?;
            
            let now = chrono::Utc::now().to_rfc3339();
            
            let provider = Provider {
                id: id.clone(),
                name,
                provider_type: pt,
                base_url,
                pylon_proxy_id: Some(proxy_id),
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.providers.insert(id.clone(), provider.clone());
            state_manager.save(&sw).await?;
            
            println!("Provider '{}' created:", provider.name);
            println!("  ID: {}", provider.id);
            println!("  Type: {}", provider.provider_type.as_str());
            println!("  Pylon Proxy: {:?}", provider.pylon_proxy_id);
        }
        
        ProviderCommands::Delete { id } => {
            let provider = sw.providers.remove(&id);
            if let Some(p) = provider {
                if let Some(proxy_id) = &p.pylon_proxy_id {
                    let _ = pylon_client.delete_proxy(proxy_id).await;
                }
                state_manager.save(&sw).await?;
                println!("Provider '{}' deleted", id);
            } else {
                eprintln!("Provider '{}' not found", id);
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}