use swarm::{Result, Skill};
use swarm::state;
use swarm::tool_manager;
use crate::cli::SkillCommands;

pub async fn handle_skill_command(command: SkillCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let mut sw = state_manager.load().await?;
    let tool_mgr = tool_manager::ToolManager::new(
        data_dir.clone(),
        sw.defaults.forgejo_url.clone(),
        sw.defaults.forgejo_token.clone(),
    );
    
    match command {
        SkillCommands::List => {
            let skills = &sw.skills;
            if skills.is_empty() {
                println!("No skills found.");
            } else {
                println!("{:<20} {:<10} {:<30}", "SLUG", "VERSION", "REPO");
                println!("{}", "-".repeat(70));
                for (slug, s) in skills {
                    println!("{:<20} {:<10} {:<30}", slug, s.version, s.forgejo_repo);
                }
            }
        }
        
        SkillCommands::Clone { slug, repo, author } => {
            if sw.skills.contains_key(&slug) {
                eprintln!("Skill '{}' already exists", slug);
                std::process::exit(1);
            }
            
            if !sw.agents.contains_key(&author) {
                eprintln!("Agent '{}' not found", author);
                std::process::exit(1);
            }
            
            println!("Cloning skill from {}...", repo);
            tool_mgr.clone_skill(&slug, &repo).await?;
            
            let metadata = tool_mgr.parse_skill_md(&slug)?;
            let git_commit = tool_mgr.get_skill_git_commit(&slug).await?;
            
            let now = chrono::Utc::now().to_rfc3339();
            
            let skill = Skill {
                slug: slug.clone(),
                name: metadata.name,
                version: metadata.version,
                description: metadata.description,
                forgejo_repo: repo,
                git_commit,
                author_agent: author,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.skills.insert(slug.clone(), skill.clone());
            state_manager.save(&sw).await?;
            
            println!("Skill '{}' cloned successfully:", skill.slug);
            println!("  Name: {}", skill.name);
            println!("  Version: {}", skill.version);
            println!("  Repo: {}", skill.forgejo_repo);
        }
        
        SkillCommands::Pull { slug } => {
            if !sw.skills.contains_key(&slug) {
                eprintln!("Skill '{}' not found", slug);
                std::process::exit(1);
            }
            
            println!("Pulling updates for skill '{}'...", slug);
            let new_commit = tool_mgr.pull_skill(&slug).await?;
            
            let metadata = tool_mgr.parse_skill_md(&slug)?;
            
            if let Some(s) = sw.skills.get_mut(&slug) {
                s.git_commit = new_commit.clone();
                s.version = metadata.version;
                s.description = metadata.description;
                s.updated_at = chrono::Utc::now().to_rfc3339();
            }
            state_manager.save(&sw).await?;
            
            println!("Skill '{}' updated to commit {}", slug, &new_commit[..8]);
        }
        
        SkillCommands::Get { slug } => {
            match sw.skills.get(&slug) {
                Some(skill) => {
                    println!("Skill: {}", skill.slug);
                    println!("  Name: {}", skill.name);
                    println!("  Version: {}", skill.version);
                    println!("  Description: {}", skill.description);
                    println!("  Author: {}", skill.author_agent);
                    println!("  Repo: {}", skill.forgejo_repo);
                    println!("  Commit: {}", &skill.git_commit[..8]);
                }
                None => {
                    eprintln!("Skill '{}' not found", slug);
                    std::process::exit(1);
                }
            }
        }
        
        SkillCommands::Delete { slug } => {
            if sw.skills.remove(&slug).is_none() {
                eprintln!("Skill '{}' not found", slug);
                std::process::exit(1);
            }
            state_manager.save(&sw).await?;
            tool_mgr.delete_skill(&slug).await?;
            println!("Skill '{}' deleted", slug);
        }
    }
    
    Ok(())
}