mod api;
mod auth;
mod agent_manager;
mod db;
mod config;
mod forgejo;
mod proxy;
mod protocol;
mod models;
mod error;

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::protocol::AgentEvent;
use std::collections::HashMap;

pub use error::{Error, Result};
pub use models::*;

pub struct AppState {
    pub db: db::Database,
    pub agent_manager: agent_manager::AgentManager,
    pub vm_connections: RwLock<HashMap<String, VmConnection>>,
    pub event_tx: broadcast::Sender<AgentEvent>,
    pub data_dir: std::path::PathBuf,
}

pub struct VmConnection {
    pub agent_name: String,
    pub connected: bool,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

const DEFAULT_PORT: u16 = 17531;

#[derive(Parser)]
#[command(name = "zerg-swarm")]
#[command(about = "Zerg Swarm - Agent cluster manager for NixOS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, global = true)]
    #[arg(value_name = "DIR")]
    #[arg(help = "Data directory (default: ~/.zerg-swarm, env: ZERG_SWARM_DATA_DIR)")]
    data_dir: Option<std::path::PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Check service status")]
    Status,
    
    #[command(about = "Start the server")]
    Serve,
    
    #[command(about = "Apply NixOS configuration")]
    Apply,
    
    #[command(about = "Agent management")]
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    
    #[command(about = "Git/Forgejo management")]
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },
    
    #[command(about = "Config management")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    
    #[command(about = "LLM Provider management")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    
    #[command(about = "API Key management")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    #[command(about = "List all agents")]
    List,
    #[command(about = "Create a new agent")]
    Create { name: String, #[arg(short, long)] forgejo_username: Option<String> },
    #[command(about = "Get agent details")]
    Get { name: String },
    #[command(about = "Delete an agent")]
    Delete { name: String },
    #[command(about = "Enable an agent")]
    Enable { name: String },
    #[command(about = "Disable an agent")]
    Disable { name: String },
}

#[derive(Subcommand)]
enum GitCommands {
    #[command(about = "Manage Forgejo accounts")]
    Account { #[command(subcommand)] command: GitAccountCommands },
    #[command(about = "Manage repositories")]
    Repo { #[command(subcommand)] command: GitRepoCommands },
    #[command(about = "Manage collaborators")]
    Collaborator { #[command(subcommand)] command: GitCollaboratorCommands },
    #[command(about = "Manage organizations")]
    Org { #[command(subcommand)] command: GitOrgCommands },
}

#[derive(Subcommand)]
enum GitAccountCommands {
    #[command(about = "Create a Forgejo account")]
    Create { #[arg(short, long)] username: String, #[arg(short, long)] password: String },
    #[command(about = "Delete a Forgejo account")]
    Delete { username: String },
    #[command(about = "List Forgejo accounts")]
    List,
    #[command(about = "Bind a Forgejo user to an agent")]
    Bind { agent: String, forgejo_user: String },
    #[command(about = "Unbind Forgejo user from an agent")]
    Unbind { agent: String },
}

#[derive(Subcommand)]
enum GitRepoCommands {
    #[command(about = "List repositories")]
    List { owner: Option<String> },
    #[command(about = "Get repository details")]
    Get { repo: String },
    #[command(about = "Create a repository")]
    Create { name: String, #[arg(short, long)] description: Option<String> },
    #[command(about = "Delete a repository")]
    Delete { repo: String },
    #[command(about = "Transfer repository ownership")]
    Transfer { repo: String, new_owner: String },
    #[command(about = "Update repository")]
    Update {
        repo: String,
        #[arg(long)]
        private: Option<bool>,
        #[arg(short, long)]
        description: Option<String>,
    },
}

#[derive(Subcommand)]
enum GitCollaboratorCommands {
    #[command(about = "List collaborators")]
    List { repo: String },
    #[command(about = "Add a collaborator")]
    Add {
        repo: String,
        username: String,
        #[arg(short, long)]
        permission: Option<String>,
    },
    #[command(about = "Remove a collaborator")]
    Remove { repo: String, username: String },
}

#[derive(Subcommand)]
enum GitOrgCommands {
    #[command(about = "List organizations")]
    List,
    #[command(about = "Create an organization")]
    Create { name: String },
    #[command(about = "Delete an organization")]
    Delete { org: String },
    #[command(about = "List organization members")]
    MemberList { org: String },
    #[command(about = "Add organization member")]
    MemberAdd { org: String, username: String },
    #[command(about = "Remove organization member")]
    MemberRemove { org: String, username: String },
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Export config to YAML")]
    Export,
    #[command(about = "Import config from YAML")]
    Import,
}

#[derive(Subcommand)]
enum ProviderCommands {
    #[command(about = "List all providers")]
    List,
    #[command(about = "Create a new provider")]
    Create {
        name: String,
        #[arg(short = 't', long)]
        provider_type: String,
        #[arg(short, long)]
        base_url: String,
        #[arg(short = 'k', long)]
        api_key: String,
    },
    #[command(about = "Get provider details")]
    Get { id: String },
    #[command(about = "Delete a provider")]
    Delete { id: String },
    #[command(about = "Enable a provider")]
    Enable { id: String },
    #[command(about = "Disable a provider")]
    Disable { id: String },
}

#[derive(Subcommand)]
enum KeyCommands {
    #[command(about = "List all API keys")]
    List,
    #[command(about = "Create a new API key")]
    Create { name: String, #[arg(short, long)] provider: String },
    #[command(about = "Delete an API key")]
    Delete { id: String },
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

fn get_data_dir(cli_data_dir: Option<std::path::PathBuf>) -> std::path::PathBuf {
    // 1. Command line option --data-dir takes precedence
    if let Some(dir) = cli_data_dir {
        return dir;
    }
    
    // 2. Environment variable ZERG_SWARM_DATA_DIR
    if let Ok(dir) = std::env::var("ZERG_SWARM_DATA_DIR") {
        return std::path::PathBuf::from(dir);
    }
    
    // 3. Default: ~/.zerg-swarm
    dirs::home_dir()
        .map(|h| h.join(".zerg-swarm"))
        .unwrap_or_else(|| std::path::PathBuf::from(".zerg-swarm"))
}

async fn init_state(data_dir: std::path::PathBuf) -> crate::Result<Arc<AppState>> {
    tokio::fs::create_dir_all(&data_dir).await?;

    let system_dir = data_dir.join("system");
    let generated_dir = system_dir.join("generated");
    tokio::fs::create_dir_all(&generated_dir).await?;

    let db_path = data_dir.join("zerg-swarm.db");
    let db = db::Database::new(&db_path).await?;

    let agent_manager = agent_manager::AgentManager::new(&system_dir);
    let (event_tx, _) = broadcast::channel::<AgentEvent>(256);

    Ok(Arc::new(AppState {
        db,
        agent_manager,
        vm_connections: RwLock::new(HashMap::new()),
        event_tx,
        data_dir,
    }))
}

async fn check_service_health(port: u16) -> (bool, String) {
    let url = format!("http://127.0.0.1:{}/health", port);
    match reqwest::Client::new().get(&url).timeout(std::time::Duration::from_secs(2)).send().await {
        Ok(resp) if resp.status().is_success() => (true, format!("running on port {}", port)),
        Ok(resp) => (false, format!("unhealthy (status {})", resp.status())),
        Err(e) => (false, format!("not running ({})", e)),
    }
}

#[tokio::main]
async fn main() -> crate::Result<()> {
    let cli = Cli::parse();
    let data_dir = get_data_dir(cli.data_dir);
    
    match cli.command {
        Commands::Status => {
            println!("Zerg Swarm Service Status\n");
            println!("{:<20} {:<10} {}", "SERVICE", "STATUS", "INFO");
            println!("{}", "-".repeat(60));
            
            let hw = check_service_health(DEFAULT_PORT).await;
            let status = if hw.0 { "\x1b[32mrunning\x1b[0m" } else { "\x1b[31mstopped\x1b[0m" };
            println!("{:<20} {:<10} {}", "zerg-swarm", status, hw.1);
        }
        
        Commands::Serve => {
            setup_logging();
            tracing::info!("Starting Zerg Swarm Manager...");
            
            let state = init_state(data_dir.clone()).await?;
            let port = std::env::var("ZERG_SWARM_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(DEFAULT_PORT);
            let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

            let username = std::env::var("ZERG_SWARM_USERNAME").unwrap_or_else(|_| "admin".to_string());
            let password = std::env::var("ZERG_SWARM_PASSWORD").unwrap_or_else(|_| "admin".to_string());
            let auth_config = auth::AuthConfig { username, password };

            api::start_server(addr, state, auth_config).await?;
        }
        
        Commands::Apply => {
            let state = init_state(data_dir.clone()).await?;
            let agents = state.db.list_agents().await?;
            let defaults = state.db.get_defaults().await;
            println!("Applying NixOS configuration...");
            state.agent_manager.apply_config(&agents, &defaults).await?;
            println!("NixOS configuration applied successfully!");
        }
        
        Commands::Agent { command } => handle_agent_command(command, data_dir.clone()).await?,
        Commands::Git { command } => handle_git_command(command, data_dir.clone()).await?,
        Commands::Config { command } => handle_config_command(command, data_dir.clone()).await?,
        Commands::Provider { command } => handle_provider_command(command, data_dir.clone()).await?,
        Commands::Key { command } => handle_key_command(command, data_dir.clone()).await?,
    }

    Ok(())
}

async fn handle_agent_command(command: AgentCommands, data_dir: std::path::PathBuf) -> crate::Result<()> {
    let state = init_state(data_dir).await?;
    match command {
        AgentCommands::List => {
            let agents = state.db.list_agents().await?;
            if agents.is_empty() {
                println!("No agents found.");
            } else {
                println!("{:<20} {:<10} {:<18} {:<18}", "NAME", "STATUS", "CONTAINER_IP", "HOST_IP");
                println!("{}", "-".repeat(70));
                for agent in agents {
                    let status = if agent.enabled { "enabled" } else { "disabled" };
                    println!("{:<20} {:<10} {:<18} {:<18}", agent.name, status, agent.container_ip, agent.host_ip);
                }
            }
        }
        AgentCommands::Create { name, forgejo_username } => {
            if state.db.get_agent(&name).await?.is_some() {
                eprintln!("Error: Agent '{}' already exists", name);
                std::process::exit(1);
            }
            let agent_num = state.db.get_next_agent_num().await?;
            let defaults = state.db.get_defaults().await;
            let now = chrono::Utc::now().to_rfc3339();
            let agent = crate::models::Agent {
                name: name.clone(), enabled: true,
                container_ip: format!("{}.{}.2", defaults.container_subnet_base, agent_num),
                host_ip: format!("{}.{}.1", defaults.container_subnet_base, agent_num),
                forgejo_username: forgejo_username.or(Some(name.clone())),
                internal_token: uuid::Uuid::new_v4().to_string(),
                created_at: now.clone(), updated_at: now,
            };
            state.db.create_agent(&agent).await?;
            println!("Agent '{}' created:", name);
            println!("  Container IP: {}", agent.container_ip);
            println!("  Host IP: {}", agent.host_ip);
        }
        AgentCommands::Get { name } => {
            match state.db.get_agent(&name).await? {
                Some(agent) => {
                    println!("Agent: {}", agent.name);
                    println!("  Status: {}", if agent.enabled { "enabled" } else { "disabled" });
                    println!("  Container IP: {}", agent.container_ip);
                    println!("  Host IP: {}", agent.host_ip);
                    println!("  Forgejo User: {}", agent.forgejo_username.clone().unwrap_or_default());
                    println!("  Token: {}", agent.internal_token);
                }
                None => { eprintln!("Error: Agent '{}' not found", name); std::process::exit(1); }
            }
        }
        AgentCommands::Delete { name } => {
            if state.db.get_agent(&name).await?.is_none() {
                eprintln!("Error: Agent '{}' not found", name); std::process::exit(1);
            }
            state.db.delete_agent(&name).await?;
            println!("Agent '{}' deleted", name);
        }
        AgentCommands::Enable { name } => {
            state.db.update_agent_enabled(&name, true).await?;
            println!("Agent '{}' enabled", name);
        }
        AgentCommands::Disable { name } => {
            state.db.update_agent_enabled(&name, false).await?;
            println!("Agent '{}' disabled", name);
        }
    }
    Ok(())
}

async fn handle_git_command(command: GitCommands, data_dir: std::path::PathBuf) -> crate::Result<()> {
    let state = init_state(data_dir).await?;
    let defaults = state.db.get_defaults().await;
    let forgejo_url = &defaults.forgejo_url;
    let forgejo_token = &defaults.forgejo_token;
    
    match command {
        GitCommands::Account { command } => match command {
            GitAccountCommands::Create { username, password } => {
                forgejo::create_user(&state.db, forgejo_url, forgejo_token, &username, &password).await?;
                println!("User '{}' created", username);
            }
            GitAccountCommands::Delete { username } => {
                forgejo::delete_user(&state.db, forgejo_url, forgejo_token, &username).await?;
                println!("User '{}' deleted", username);
            }
            GitAccountCommands::List => {
                let users = forgejo::list_users(&state.db).await?;
                println!("{:<20} {:<30} {}", "USERNAME", "EMAIL", "BOUND AGENT");
                println!("{}", "-".repeat(70));
                for user in users {
                    let bound = state.db.get_agent_by_forgejo_user(&user.username).await?
                        .map(|a| a.name).unwrap_or_else(|| "-".to_string());
                    println!("{:<20} {:<30} {}", user.username, user.email, bound);
                }
            }
            GitAccountCommands::Bind { agent, forgejo_user } => {
                state.db.bind_forgejo_user(&agent, &forgejo_user).await?;
                println!("Agent '{}' bound to Forgejo user '{}'", agent, forgejo_user);
            }
            GitAccountCommands::Unbind { agent } => {
                state.db.unbind_forgejo_user(&agent).await?;
                println!("Agent '{}' unbound from Forgejo user", agent);
            }
        },
        GitCommands::Repo { command } => match command {
            GitRepoCommands::List { owner } => {
                let repos = forgejo::list_repos(forgejo_url, forgejo_token, owner.as_deref()).await?;
                println!("{:<30} {:<15} {:<10}", "NAME", "OWNER", "PRIVATE");
                println!("{}", "-".repeat(60));
                for r in repos {
                    println!("{:<30} {:<15} {:<10}", r.name, r.owner.login, if r.private { "yes" } else { "no" });
                }
            }
            GitRepoCommands::Get { repo } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                match forgejo::get_repo(forgejo_url, forgejo_token, parts[0], parts[1]).await? {
                    Some(r) => {
                        println!("Repository: {}", r.full_name);
                        println!("  Description: {}", r.description);
                        println!("  Private: {}", r.private);
                        println!("  Default branch: {}", r.default_branch);
                        println!("  Stars: {} | Forks: {} | Issues: {}", r.stars_count, r.forks_count, r.open_issues_count);
                    }
                    None => { eprintln!("Error: Repository '{}' not found", repo); std::process::exit(1); }
                }
            }
            GitRepoCommands::Create { name, description } => {
                let repo = forgejo::create_repo(forgejo_url, forgejo_token, &name, description.as_deref()).await?;
                println!("Repository '{}' created: {}", repo.name, repo.html_url);
            }
            GitRepoCommands::Delete { repo } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                if !confirm(&format!("Delete repository '{}'?", repo)) { return Ok(()); }
                forgejo::delete_repo(forgejo_url, forgejo_token, parts[0], parts[1]).await?;
                println!("Repository '{}' deleted", repo);
            }
            GitRepoCommands::Transfer { repo, new_owner } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                forgejo::transfer_repo(forgejo_url, forgejo_token, parts[0], parts[1], &new_owner).await?;
                println!("Repository '{}' transferred to '{}'", repo, new_owner);
            }
            GitRepoCommands::Update { repo, private, description } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                let r = forgejo::update_repo(forgejo_url, forgejo_token, parts[0], parts[1], private, description.as_deref()).await?;
                println!("Repository '{}' updated", r.full_name);
            }
        },
        GitCommands::Collaborator { command } => match command {
            GitCollaboratorCommands::List { repo } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                let collaborators = forgejo::list_collaborators(forgejo_url, forgejo_token, parts[0], parts[1]).await?;
                println!("{:<20} {:<10} {:<10} {:<10}", "USERNAME", "ADMIN", "WRITE", "READ");
                println!("{}", "-".repeat(50));
                for c in collaborators {
                    println!("{:<20} {:<10} {:<10} {:<10}", 
                        c.login, 
                        if c.permissions.admin { "yes" } else { "no" },
                        if c.permissions.push { "yes" } else { "no" },
                        if c.permissions.pull { "yes" } else { "no" });
                }
            }
            GitCollaboratorCommands::Add { repo, username, permission } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                forgejo::add_collaborator(forgejo_url, forgejo_token, parts[0], parts[1], &username, permission.as_deref()).await?;
                println!("Collaborator '{}' added to '{}'", username, repo);
            }
            GitCollaboratorCommands::Remove { repo, username } => {
                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 { eprintln!("Error: Use format 'owner/repo'"); std::process::exit(1); }
                forgejo::remove_collaborator(forgejo_url, forgejo_token, parts[0], parts[1], &username).await?;
                println!("Collaborator '{}' removed from '{}'", username, repo);
            }
        },
        GitCommands::Org { command } => match command {
            GitOrgCommands::List => {
                let orgs = forgejo::list_orgs(forgejo_url, forgejo_token).await?;
                println!("{:<20} {:<30}", "NAME", "FULL NAME");
                println!("{}", "-".repeat(50));
                for o in orgs {
                    println!("{:<20} {:<30}", o.login(), o.full_name);
                }
            }
            GitOrgCommands::Create { name } => {
                let org = forgejo::create_org(forgejo_url, forgejo_token, &name).await?;
                println!("Organization '{}' created", org.login());
            }
            GitOrgCommands::Delete { org } => {
                if !confirm(&format!("Delete organization '{}'?", org)) { return Ok(()); }
                forgejo::delete_org(forgejo_url, forgejo_token, &org).await?;
                println!("Organization '{}' deleted", org);
            }
            GitOrgCommands::MemberList { org } => {
                let members = forgejo::list_org_members(forgejo_url, forgejo_token, &org).await?;
                println!("Members of '{}':", org);
                for m in members {
                    println!("  - {} ({})", m.login, m.full_name);
                }
            }
            GitOrgCommands::MemberAdd { org, username } => {
                forgejo::add_org_member(forgejo_url, forgejo_token, &org, &username).await?;
                println!("Member '{}' added to '{}'", username, org);
            }
            GitOrgCommands::MemberRemove { org, username } => {
                forgejo::remove_org_member(forgejo_url, forgejo_token, &org, &username).await?;
                println!("Member '{}' removed from '{}'", username, org);
            }
        },
    }
    Ok(())
}

fn confirm(msg: &str) -> bool {
    use std::io::Write;
    print!("{} (y/N): ", msg);
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_lowercase() == "y"
}

async fn handle_config_command(command: ConfigCommands, data_dir: std::path::PathBuf) -> crate::Result<()> {
    let state = init_state(data_dir.clone()).await?;
    match command {
        ConfigCommands::Export => {
            let export_path = data_dir.join("config.yaml");
            config::export_to_yaml(&state.db, &export_path).await?;
            println!("Config exported to {:?}", export_path);
        }
        ConfigCommands::Import => {
            let import_path = data_dir.join("config.yaml");
            config::import_from_yaml(&state.db, &import_path).await?;
            println!("Config imported from {:?}", import_path);
        }
    }
    Ok(())
}

async fn handle_provider_command(command: ProviderCommands, data_dir: std::path::PathBuf) -> crate::Result<()> {
    let state = init_state(data_dir).await?;
    match command {
        ProviderCommands::List => {
            let providers = state.db.list_providers().await?;
            if providers.is_empty() {
                println!("No providers found.");
            } else {
                println!("{:<40} {:<15} {:<10} {:<40}", "ID", "NAME", "TYPE", "BASE_URL");
                println!("{}", "-".repeat(110));
                for p in providers {
                    let status = if p.enabled { "enabled" } else { "disabled" };
                    println!("{:<40} {:<15} {:<10} {:<40}", p.id, p.name, p.provider_type.as_str(), p.base_url);
                    println!("{:<40} {:<15} {:<10}", "", "", status);
                }
            }
        }
        ProviderCommands::Create { name, provider_type, base_url, api_key } => {
            let pt = crate::models::ProviderType::from_str(&provider_type)
                .ok_or_else(|| crate::Error::Validation(format!("Invalid provider type: {}", provider_type)))?;
            let req = crate::models::CreateProviderRequest { name, provider_type: pt, base_url, api_key };
            let provider = state.db.create_provider(&req).await?;
            println!("Provider '{}' created:", provider.name);
            println!("  ID: {}", provider.id);
            println!("  Type: {}", provider.provider_type.as_str());
            println!("  Base URL: {}", provider.base_url);
        }
        ProviderCommands::Get { id } => {
            match state.db.get_provider(&id).await? {
                Some(p) => {
                    println!("Provider: {}", p.name);
                    println!("  ID: {}", p.id);
                    println!("  Type: {}", p.provider_type.as_str());
                    println!("  Base URL: {}", p.base_url);
                    println!("  Status: {}", if p.enabled { "enabled" } else { "disabled" });
                }
                None => { eprintln!("Error: Provider '{}' not found", id); std::process::exit(1); }
            }
        }
        ProviderCommands::Delete { id } => {
            state.db.delete_provider(&id).await?;
            println!("Provider '{}' deleted", id);
        }
        ProviderCommands::Enable { id } => {
            state.db.update_provider_enabled(&id, true).await?;
            println!("Provider '{}' enabled", id);
        }
        ProviderCommands::Disable { id } => {
            state.db.update_provider_enabled(&id, false).await?;
            println!("Provider '{}' disabled", id);
        }
    }
    Ok(())
}

async fn handle_key_command(command: KeyCommands, data_dir: std::path::PathBuf) -> crate::Result<()> {
    let state = init_state(data_dir).await?;
    match command {
        KeyCommands::List => {
            let keys = state.db.list_api_keys().await?;
            let providers = state.db.list_providers().await?;
            let provider_map: HashMap<_, _> = providers.into_iter().map(|p| (p.id, p.name)).collect();
            let unknown = "unknown".to_string();
            
            if keys.is_empty() {
                println!("No API keys found.");
            } else {
                println!("{:<40} {:<20} {:<20}", "ID", "NAME", "PROVIDER");
                println!("{}", "-".repeat(90));
                for k in keys {
                    let provider_name = provider_map.get(&k.provider_id).unwrap_or(&unknown);
                    println!("{:<40} {:<20} {:<20}", k.id, k.name, provider_name);
                }
            }
        }
        KeyCommands::Create { name, provider } => {
            let req = crate::models::CreateApiKeyRequest { name, provider_id: provider };
            let (key, raw_key) = state.db.create_api_key(&req).await?;
            println!("API key '{}' created:", key.name);
            println!("  ID: {}", key.id);
            println!("  Key: {}", raw_key);
            println!("\n\x1b[33mWarning: Save this key now, it won't be shown again!\x1b[0m");
        }
        KeyCommands::Delete { id } => {
            state.db.delete_api_key(&id).await?;
            println!("API key '{}' deleted", id);
        }
    }
    Ok(())
}