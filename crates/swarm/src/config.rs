use crate::models::Config;
use crate::Result;
use std::path::Path;
use crate::db::Database;

pub async fn export_to_yaml(db: &Database, path: &Path) -> Result<()> {
    let agents = db.list_agents().await?;
    let forgejo_users = db.list_forgejo_users().await?;
    let defaults = db.get_defaults().await;
    
    let mut agents_map = std::collections::HashMap::new();
    for agent in agents {
        agents_map.insert(agent.name.clone(), agent);
    }
    
    let mut users_map = std::collections::HashMap::new();
    for user in forgejo_users {
        users_map.insert(user.username.clone(), user);
    }
    
    let config = Config {
        version: "1.0".to_string(),
        defaults,
        agents: agents_map,
        forgejo_users: users_map,
    };
    
    let content = serde_yaml::to_string(&config)?;
    tokio::fs::write(path, content).await?;
    
    Ok(())
}

pub async fn import_from_yaml(db: &Database, path: &Path) -> Result<()> {
    let content = tokio::fs::read_to_string(path).await?;
    let config: Config = serde_yaml::from_str(&content)?;
    
    db.update_defaults(&config.defaults).await?;
    
    for (_, agent) in config.agents {
        if db.get_agent(&agent.name).await?.is_none() {
            db.create_agent(&agent).await?;
        }
    }
    
    for (_, user) in config.forgejo_users {
        if db.get_forgejo_user(&user.username).await?.is_none() {
            db.create_forgejo_user(&user).await?;
        }
    }
    
    Ok(())
}

pub async fn export_config(db: &Database) -> Result<Config> {
    let agents = db.list_agents().await?;
    let forgejo_users = db.list_forgejo_users().await?;
    let defaults = db.get_defaults().await;
    
    let mut agents_map = std::collections::HashMap::new();
    for agent in agents {
        agents_map.insert(agent.name.clone(), agent);
    }
    
    let mut users_map = std::collections::HashMap::new();
    for user in forgejo_users {
        users_map.insert(user.username.clone(), user);
    }
    
    Ok(Config {
        version: "1.0".to_string(),
        defaults,
        agents: agents_map,
        forgejo_users: users_map,
    })
}