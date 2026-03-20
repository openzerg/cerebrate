use cerebrate::{Result, Tool, Error};
use cerebrate::state;
use cerebrate::tool_manager;
use crate::cli::ToolCommands;

pub async fn handle_tool_command(command: ToolCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    let tool_mgr = tool_manager::ToolManager::new(
        data_dir.clone(),
        sw.defaults.forgejo_url.clone(),
        sw.defaults.forgejo_token.clone(),
    );
    
    match command {
        ToolCommands::List => {
            let tools = &sw.tools;
            if tools.is_empty() {
                println!("No tools found.");
            } else {
                println!("{:<20} {:<10} {:<30} {:<8}", "SLUG", "VERSION", "REPO", "ENABLED");
                println!("{}", "-".repeat(80));
                for (slug, t) in tools {
                    println!("{:<20} {:<10} {:<30} {:<8}", 
                        slug, 
                        t.version, 
                        t.forgejo_repo,
                        if t.enabled { "yes" } else { "no" }
                    );
                }
            }
        }
        
        ToolCommands::Clone { slug, repo, author } => {
            if sw.tools.contains_key(&slug) {
                eprintln!("Tool '{}' already exists", slug);
                std::process::exit(1);
            }
            
            if !sw.agents.contains_key(&author) {
                eprintln!("Agent '{}' not found", author);
                std::process::exit(1);
            }
            
            println!("Cloning tool from {}...", repo);
            tool_mgr.clone_tool(&slug, &repo).await?;
            
            let metadata = tool_mgr.parse_tool_md(&slug)?;
            let git_commit = tool_mgr.get_git_commit(&slug).await?;
            
            let now = chrono::Utc::now().to_rfc3339();
            
            let tool = Tool {
                slug: slug.clone(),
                name: metadata.name,
                version: metadata.version,
                description: metadata.description,
                forgejo_repo: repo,
                git_commit,
                entrypoint: metadata.entrypoint,
                input_schema: metadata.input_schema,
                output_schema: metadata.output_schema,
                author_agent: author.clone(),
                allowed_agents: vec![author],
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.tools.insert(slug.clone(), tool.clone());
            state_manager.save(&sw).await?;
            
            println!("Tool '{}' cloned successfully:", tool.slug);
            println!("  Name: {}", tool.name);
            println!("  Version: {}", tool.version);
            println!("  Repo: {}", tool.forgejo_repo);
            println!("  Entrypoint: {}", tool.entrypoint);
        }
        
        ToolCommands::Pull { slug } => {
            if !sw.tools.contains_key(&slug) {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
            
            println!("Pulling updates for tool '{}'...", slug);
            let new_commit = tool_mgr.pull_tool(&slug).await?;
            
            let metadata = tool_mgr.parse_tool_md(&slug)?;
            
            if let Some(t) = sw.tools.get_mut(&slug) {
                t.git_commit = new_commit.clone();
                t.version = metadata.version;
                t.description = metadata.description;
                t.entrypoint = metadata.entrypoint;
                t.input_schema = metadata.input_schema;
                t.output_schema = metadata.output_schema;
                t.updated_at = chrono::Utc::now().to_rfc3339();
            }
            state_manager.save(&sw).await?;
            
            println!("Tool '{}' updated to commit {}", slug, &new_commit[..8]);
        }
        
        ToolCommands::Get { slug } => {
            match sw.tools.get(&slug) {
                Some(tool) => {
                    println!("Tool: {}", tool.slug);
                    println!("  Name: {}", tool.name);
                    println!("  Version: {}", tool.version);
                    println!("  Description: {}", tool.description);
                    println!("  Author: {}", tool.author_agent);
                    println!("  Repo: {}", tool.forgejo_repo);
                    println!("  Commit: {}", &tool.git_commit[..8]);
                    println!("  Entrypoint: {}", tool.entrypoint);
                    println!("  Enabled: {}", tool.enabled);
                    println!("  Allowed agents: {:?}", tool.allowed_agents);
                    
                    let env_vars = tool_mgr.list_env(&slug).await?;
                    if !env_vars.is_empty() {
                        println!("  Env vars: {:?}", env_vars);
                    }
                }
                None => {
                    eprintln!("Tool '{}' not found", slug);
                    std::process::exit(1);
                }
            }
        }
        
        ToolCommands::Delete { slug } => {
            if sw.tools.remove(&slug).is_none() {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
            state_manager.save(&sw).await?;
            tool_mgr.delete_tool(&slug).await?;
            println!("Tool '{}' deleted", slug);
        }
        
        ToolCommands::Authorize { slug, agent_name } => {
            if let Some(tool) = sw.tools.get_mut(&slug) {
                if !tool.allowed_agents.contains(&agent_name) {
                    tool.allowed_agents.push(agent_name.clone());
                    tool.updated_at = chrono::Utc::now().to_rfc3339();
                    let tool_slug = tool.slug.clone();
                    state_manager.save(&sw).await?;
                    println!("Agent '{}' authorized for tool '{}'", agent_name, tool_slug);
                } else {
                    let tool_slug = tool.slug.clone();
                    println!("Agent '{}' already authorized for tool '{}'", agent_name, tool_slug);
                }
            } else {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
        }
        
        ToolCommands::Revoke { slug, agent_name } => {
            if let Some(tool) = sw.tools.get_mut(&slug) {
                tool.allowed_agents.retain(|a| a != &agent_name);
                tool.updated_at = chrono::Utc::now().to_rfc3339();
                let tool_slug = tool.slug.clone();
                state_manager.save(&sw).await?;
                println!("Agent '{}' revoked from tool '{}'", agent_name, tool_slug);
            } else {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
        }
        
        ToolCommands::SetEnv { slug, key, value } => {
            if !sw.tools.contains_key(&slug) {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
            tool_mgr.set_env(&slug, &key, &value).await?;
            println!("Env '{}' set for tool '{}'", key, slug);
        }
        
        ToolCommands::ListEnv { slug } => {
            if !sw.tools.contains_key(&slug) {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
            let env_vars = tool_mgr.list_env(&slug).await?;
            if env_vars.is_empty() {
                println!("No env vars found for tool '{}'", slug);
            } else {
                println!("Env vars for tool '{}':", slug);
                for key in env_vars {
                    println!("  - {}", key);
                }
            }
        }
        
        ToolCommands::DeleteEnv { slug, key } => {
            if !sw.tools.contains_key(&slug) {
                eprintln!("Tool '{}' not found", slug);
                std::process::exit(1);
            }
            tool_mgr.delete_env(&slug, &key).await?;
            println!("Env '{}' deleted for tool '{}'", key, slug);
        }
        
        ToolCommands::Invoke { slug, input } => {
            let tool = sw.tools.get(&slug)
                .ok_or_else(|| Error::NotFound(format!("Tool '{}' not found", slug)))?
                .clone();
            
            let input_json: serde_json::Value = serde_json::from_str(&input)
                .map_err(|e| Error::Validation(format!("Invalid JSON input: {}", e)))?;
            
            let response = tool_mgr.invoke_host_tool(&tool, &input_json).await?;
            
            if response.success {
                println!("Tool '{}' executed successfully:", tool.slug);
                if let Some(output) = response.output {
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
            } else {
                eprintln!("Tool '{}' execution failed:", tool.slug);
                if let Some(error) = response.error {
                    eprintln!("{}", error);
                }
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}