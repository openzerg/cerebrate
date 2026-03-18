mod api;
mod auth;
mod agent_manager;
mod state;
mod checkpoint;
mod btrfs;
mod config;
mod forgejo;
mod proxy;
mod protocol;
mod models;
mod error;
mod tool_manager;

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::protocol::AgentEvent;
use std::collections::HashMap;
use tokio::sync::oneshot;

pub use error::{Error, Result};
pub use models::*;

pub type PendingToolResults = RwLock<HashMap<String, oneshot::Sender<InvokeToolResponse>>>;

pub struct AppState {
    pub state_manager: state::StateManager,
    pub agent_manager: agent_manager::AgentManager,
    pub tool_manager: tool_manager::ToolManager,
    pub vm_connections: RwLock<HashMap<String, VmConnection>>,
    pub pending_tool_results: PendingToolResults,
    pub event_tx: broadcast::Sender<AgentEvent>,
    pub data_dir: std::path::PathBuf,
    pub apply_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

pub struct VmConnection {
    pub agent_name: String,
    pub connected: bool,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

const DEFAULT_PORT: u16 = 17531;
const MAX_CHECKPOINTS_PER_AGENT: usize = 10;

#[derive(Parser)]
#[command(name = "zerg-swarm")]
#[command(about = "Zerg Swarm - Agent cluster manager for NixOS")]
#[command(version)]
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
    Apply {
        #[arg(short, long)]
        #[arg(help = "Template directory (default: data-dir/system)")]
        template: Option<std::path::PathBuf>,
        
        #[arg(long, default_value = "/dev/sda2")]
        #[arg(help = "Btrfs device for agent filesystems")]
        btrfs_device: String,
    },
    
    #[command(about = "Agent management")]
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    
    #[command(about = "Checkpoint management")]
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
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
    
    #[command(about = "Skill library management")]
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },
    
    #[command(about = "Tool library management")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    #[command(about = "List all agents")]
    List,
    
    #[command(about = "Create a new agent")]
    Create { 
        name: String, 
        #[arg(short, long)] 
        forgejo_username: Option<String> 
    },
    
    #[command(about = "Get agent details")]
    Get { name: String },
    
    #[command(about = "Delete an agent")]
    Delete { name: String },
    
    #[command(about = "Enable an agent")]
    Enable { name: String },
    
    #[command(about = "Disable an agent")]
    Disable { name: String },
    
    #[command(about = "Create a checkpoint")]
    Checkpoint { 
        name: String,
        #[arg(short, long)]
        #[arg(help = "Checkpoint description")]
        desc: Option<String>,
    },
    
    #[command(about = "Rollback to a checkpoint")]
    Rollback { 
        name: String,
        checkpoint_id: String,
    },
    
    #[command(about = "List checkpoints for an agent")]
    ListCheckpoints { name: String },
    
    #[command(about = "Delete a checkpoint")]
    DeleteCheckpoint { checkpoint_id: String },
}

#[derive(Subcommand)]
enum CheckpointCommands {
    #[command(about = "Clone a checkpoint to a new agent")]
    Clone {
        checkpoint_id: String,
        new_name: String,
    },
    
    #[command(about = "List all checkpoints")]
    List {
        #[arg(short, long)]
        #[arg(help = "Filter by agent name")]
        agent: Option<String>,
    },
}

#[derive(Subcommand)]
enum GitCommands {
    #[command(about = "List Forgejo users")]
    Users,
    
    #[command(about = "Create a Forgejo user")]
    CreateUser { username: String, password: String, email: String },
    
    #[command(about = "Delete a Forgejo user")]
    DeleteUser { username: String },
    
    #[command(about = "List organizations")]
    Orgs,
    
    #[command(about = "Create an organization")]
    CreateOrg { name: String },
    
    #[command(about = "Delete an organization")]
    DeleteOrg { name: String },
    
    #[command(about = "List repositories")]
    Repos,
    
    #[command(about = "Create a repository")]
    CreateRepo { owner: String, name: String },
    
    #[command(about = "Delete a repository")]
    DeleteRepo { owner: String, name: String },
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
    #[command(about = "List LLM providers")]
    List,
    
    #[command(about = "Create a new provider")]
    Create {
        name: String,
        provider_type: String,
        base_url: String,
        api_key: String,
    },
    
    #[command(about = "Delete a provider")]
    Delete { id: String },
}

#[derive(Subcommand)]
enum KeyCommands {
    #[command(about = "List API keys")]
    List,
    
    #[command(about = "Create a new API key")]
    Create {
        name: String,
        #[arg(short, long)]
        provider: String,
    },
    
    #[command(about = "Delete an API key")]
    Delete { id: String },
}

#[derive(Subcommand)]
enum SkillCommands {
    #[command(about = "List all skills")]
    List,
    
    #[command(about = "Clone a skill from Forgejo")]
    Clone {
        slug: String,
        #[arg(short, long)]
        #[arg(help = "Forgejo repository path (e.g., skills/excel-xlsx)")]
        repo: String,
        #[arg(short, long)]
        #[arg(help = "Author agent name")]
        author: String,
    },
    
    #[command(about = "Pull latest changes for a skill")]
    Pull { slug: String },
    
    #[command(about = "Get skill details")]
    Get { slug: String },
    
    #[command(about = "Delete a skill")]
    Delete { slug: String },
}

#[derive(Subcommand)]
enum ToolCommands {
    #[command(about = "List all tools")]
    List,
    
    #[command(about = "Clone a tool from Forgejo")]
    Clone {
        slug: String,
        #[arg(short, long)]
        #[arg(help = "Forgejo repository path (e.g., tools/brave-search)")]
        repo: String,
        #[arg(short, long)]
        #[arg(help = "Author agent name")]
        author: String,
    },
    
    #[command(about = "Pull latest changes for a tool")]
    Pull { slug: String },
    
    #[command(about = "Get tool details")]
    Get { slug: String },
    
    #[command(about = "Delete a tool")]
    Delete { slug: String },
    
    #[command(about = "Authorize an agent to use a tool")]
    Authorize {
        slug: String,
        agent_name: String,
    },
    
    #[command(about = "Revoke an agent's access to a tool")]
    Revoke {
        slug: String,
        agent_name: String,
    },
    
    #[command(about = "Set an environment variable for a tool")]
    SetEnv {
        slug: String,
        key: String,
        value: String,
    },
    
    #[command(about = "List environment variables for a tool")]
    ListEnv { slug: String },
    
    #[command(about = "Delete an environment variable for a tool")]
    DeleteEnv {
        slug: String,
        key: String,
    },
    
    #[command(about = "Invoke a tool")]
    Invoke {
        slug: String,
        #[arg(short, long)]
        #[arg(help = "Caller agent name")]
        caller: String,
        #[arg(short, long)]
        #[arg(help = "JSON input")]
        input: String,
    },
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

fn get_data_dir(cli_data_dir: Option<std::path::PathBuf>) -> std::path::PathBuf {
    if let Some(dir) = cli_data_dir {
        return dir;
    }
    
    if let Ok(dir) = std::env::var("ZERG_SWARM_DATA_DIR") {
        return std::path::PathBuf::from(dir);
    }
    
    dirs::home_dir()
        .map(|h| h.join(".zerg-swarm"))
        .unwrap_or_else(|| std::path::PathBuf::from(".zerg-swarm"))
}

async fn init_state(data_dir: std::path::PathBuf) -> Result<Arc<AppState>> {
    tokio::fs::create_dir_all(&data_dir).await?;

    let system_dir = data_dir.join("system");
    let generated_dir = system_dir.join("generated");
    tokio::fs::create_dir_all(&generated_dir).await?;

    let state_manager = state::StateManager::new(&data_dir);
    let agent_manager = agent_manager::AgentManager::new(&system_dir);
    
    let sw = state_manager.load().await?;
    let forgejo_url = sw.defaults.forgejo_url.clone();
    let forgejo_token = sw.defaults.forgejo_token.clone();
    
    let tool_manager = tool_manager::ToolManager::new(data_dir.clone(), forgejo_url, forgejo_token);
    let (event_tx, _) = broadcast::channel::<AgentEvent>(256);
    let (apply_tx, apply_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let state = Arc::new(AppState {
        state_manager,
        agent_manager,
        tool_manager,
        vm_connections: RwLock::new(HashMap::new()),
        pending_tool_results: RwLock::new(HashMap::new()),
        event_tx,
        data_dir: data_dir.clone(),
        apply_tx,
    });

    // Spawn apply task handler
    let state_clone = state.clone();
    tokio::spawn(async move {
        handle_apply_tasks(state_clone, apply_rx).await;
    });

    Ok(state)
}

async fn handle_apply_tasks(
    state: Arc<AppState>,
    mut apply_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
) {
    let btrfs_device = std::env::var("ZERG_SWARM_BTRFS_DEVICE")
        .unwrap_or_else(|_| "/dev/sda2".to_string());
    
    while let Some(_) = apply_rx.recv().await {
        tracing::info!("Applying NixOS configuration...");
        
        // Notify that we're starting
        let _ = state.event_tx.send(protocol::AgentEvent {
            event: protocol::AgentEventType::ConfigApplying,
            agent_name: "system".to_string(),
            timestamp: chrono::Utc::now(),
            data: None,
        });
        
        match state.state_manager.load().await {
            Ok(sw) => {
                if let Err(e) = state.agent_manager.apply(&sw, &btrfs_device).await {
                    tracing::error!("Failed to apply configuration: {}", e);
                    // Notify error via WebSocket
                    let _ = state.event_tx.send(protocol::AgentEvent {
                        event: protocol::AgentEventType::ConfigError,
                        agent_name: "system".to_string(),
                        timestamp: chrono::Utc::now(),
                        data: Some(serde_json::json!({ "error": e.to_string() })),
                    });
                } else {
                    tracing::info!("NixOS configuration applied successfully");
                    // Notify success via WebSocket
                    let _ = state.event_tx.send(protocol::AgentEvent {
                        event: protocol::AgentEventType::ConfigApplied,
                        agent_name: "system".to_string(),
                        timestamp: chrono::Utc::now(),
                        data: None,
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to load state: {}", e);
                let _ = state.event_tx.send(protocol::AgentEvent {
                    event: protocol::AgentEventType::ConfigError,
                    agent_name: "system".to_string(),
                    timestamp: chrono::Utc::now(),
                    data: Some(serde_json::json!({ "error": e.to_string() })),
                });
            }
        }
        
        // Debounce: wait a bit to coalesce multiple requests
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Drain any pending requests during the rebuild
        while apply_rx.try_recv().is_ok() {}
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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
        
        Commands::Apply { template, btrfs_device } => {
            let state = init_state(data_dir.clone()).await?;
            let sw = state.state_manager.load().await?;
            
            let template_dir = template.unwrap_or_else(|| data_dir.join("system"));
            let manager = state.agent_manager.clone().with_template(&template_dir);
            
            println!("Applying NixOS configuration...");
            manager.apply(&sw, &btrfs_device).await?;
            println!("NixOS configuration applied successfully!");
        }
        
        Commands::Agent { command } => handle_agent_command(command, data_dir.clone()).await?,
        Commands::Checkpoint { command } => handle_checkpoint_command(command, data_dir.clone()).await?,
        Commands::Git { command } => handle_git_command(command, data_dir.clone()).await?,
        Commands::Config { command } => handle_config_command(command, data_dir.clone()).await?,
        Commands::Provider { command } => handle_provider_command(command, data_dir.clone()).await?,
        Commands::Key { command } => handle_key_command(command, data_dir.clone()).await?,
        Commands::Skill { command } => handle_skill_command(command, data_dir.clone()).await?,
        Commands::Tool { command } => handle_tool_command(command, data_dir.clone()).await?,
    }

    Ok(())
}

async fn check_service_health(port: u16) -> (bool, String) {
    let url = format!("http://localhost:{}/api/health", port);
    match reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => (true, format!("port {}", port)),
        Ok(resp) => (false, format!("HTTP {}", resp.status())),
        Err(e) => (false, e.to_string()),
    }
}

async fn handle_agent_command(command: AgentCommands, data_dir: std::path::PathBuf) -> Result<()> {
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

async fn handle_checkpoint_command(command: CheckpointCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let checkpoint_mgr = checkpoint::CheckpointManager::new(&data_dir, "/dev/sda2", std::path::Path::new("/home"));
    
    match command {
        CheckpointCommands::Clone { checkpoint_id, new_name } => {
            checkpoint_mgr.clone(&checkpoint_id, &new_name).await?;
            println!("Cloned checkpoint '{}' to new agent '{}'", checkpoint_id, new_name);
        }
        
        CheckpointCommands::List { agent } => {
            let checkpoints = checkpoint_mgr.list_checkpoints(agent.as_deref()).await?;
            
            if checkpoints.is_empty() {
                println!("No checkpoints found");
            } else {
                println!("{:<30} {:<15} {:<20} {}", "ID", "AGENT", "CREATED", "DESCRIPTION");
                println!("{}", "-".repeat(85));
                for cp in checkpoints {
                    println!("{:<30} {:<15} {:<20} {}", cp.id, cp.agent_name, cp.created_at, cp.description);
                }
            }
        }
    }
    
    Ok(())
}

async fn handle_git_command(command: GitCommands, data_dir: std::path::PathBuf) -> Result<()> {
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

async fn handle_config_command(command: ConfigCommands, data_dir: std::path::PathBuf) -> Result<()> {
    let state_manager = state::StateManager::new(&data_dir);
    let sw = state_manager.load().await?;
    
    match command {
        ConfigCommands::Export => {
            let export_path = data_dir.join("config.yaml");
            config::export_to_yaml(&sw, &export_path).await?;
            println!("Config exported to {:?}", export_path);
        }
        ConfigCommands::Import => {
            let import_path = data_dir.join("config.yaml");
            let imported = config::import_from_yaml(&import_path).await?;
            state_manager.save(&imported).await?;
            println!("Config imported from {:?}", import_path);
        }
    }
    Ok(())
}

async fn handle_provider_command(command: ProviderCommands, data_dir: std::path::PathBuf) -> Result<()> {
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

async fn handle_key_command(command: KeyCommands, data_dir: std::path::PathBuf) -> Result<()> {
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

async fn handle_skill_command(command: SkillCommands, data_dir: std::path::PathBuf) -> Result<()> {
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

async fn handle_tool_command(command: ToolCommands, data_dir: std::path::PathBuf) -> Result<()> {
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
        
        ToolCommands::Invoke { slug, caller, input } => {
            let tool = sw.tools.get(&slug)
                .ok_or_else(|| Error::NotFound(format!("Tool '{}' not found", slug)))?
                .clone();
            
            if !tool_mgr.check_authorization(&tool, &caller) {
                eprintln!("Agent '{}' is not authorized to invoke tool '{}'", caller, slug);
                std::process::exit(1);
            }
            
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