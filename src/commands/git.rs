use cerebrate::Result;
use cerebrate::state;
use cerebrate::forgejo;
use crate::cli::GitCommands;

pub async fn handle_git_command(command: GitCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let sw = state_manager.load().await?;
    let defaults = &sw.defaults;
    let forgejo_url = &defaults.forgejo_url;
    let forgejo_token = &defaults.forgejo_token;
    
    match command {
        GitCommands::Users => {
            let users = forgejo::user::list_users(forgejo_url, forgejo_token).await?;
            println!("{:<20} {:<30}", "USERNAME", "EMAIL");
            println!("{}", "-".repeat(50));
            for user in users {
                println!("{:<20} {:<30}", user.login, user.email);
            }
        }
        
        GitCommands::CreateUser { username, password, email } => {
            forgejo::user::create_user(forgejo_url, forgejo_token, &username, &password, &email).await?;
            println!("User '{}' created", username);
        }
        
        GitCommands::DeleteUser { username } => {
            forgejo::user::delete_user(forgejo_url, forgejo_token, &username).await?;
            println!("User '{}' deleted", username);
        }
        
        GitCommands::Orgs => {
            let orgs = forgejo::org::list_orgs(forgejo_url, forgejo_token).await?;
            println!("{:<20} {:<30}", "NAME", "FULL NAME");
            println!("{}", "-".repeat(50));
            for org in orgs {
                println!("{:<20} {:<30}", org.username, org.full_name);
            }
        }
        
        GitCommands::CreateOrg { name } => {
            forgejo::org::create_org(forgejo_url, forgejo_token, &name).await?;
            println!("Organization '{}' created", name);
        }
        
        GitCommands::DeleteOrg { name } => {
            forgejo::org::delete_org(forgejo_url, forgejo_token, &name).await?;
            println!("Organization '{}' deleted", name);
        }
        
        GitCommands::Repos => {
            let repos = forgejo::repo::list_repos(forgejo_url, forgejo_token, None).await?;
            println!("{:<30} {:<20} {:<10}", "NAME", "OWNER", "PRIVATE");
            println!("{}", "-".repeat(60));
            for repo in repos {
                println!("{:<30} {:<20} {:<10}", repo.name, repo.owner.login, if repo.private { "yes" } else { "no" });
            }
        }
        
        GitCommands::CreateRepo { owner, name } => {
            forgejo::repo::create_repo(forgejo_url, forgejo_token, &owner, &name).await?;
            println!("Repository '{}/{}' created", owner, name);
        }
        
        GitCommands::DeleteRepo { owner, name } => {
            forgejo::repo::delete_repo(forgejo_url, forgejo_token, &owner, &name).await?;
            println!("Repository '{}/{}' deleted", owner, name);
        }
    }
    
    Ok(())
}