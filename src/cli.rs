use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub const DEFAULT_PORT: u16 = 17531;
#[allow(dead_code)]
pub const MAX_CHECKPOINTS_PER_AGENT: usize = 10;

#[derive(Parser)]
#[command(name = "cerebrate")]
#[command(about = "Cerebrate - Agent cluster manager for NixOS")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    #[arg(value_name = "DIR")]
    #[arg(help = "Data directory (default: ~/.cerebrate, env: CEREBRATE_DATA_DIR)")]
    pub data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Check service status")]
    Status,

    #[command(about = "Start the server")]
    Serve,

    #[command(about = "Apply Incus container configuration")]
    Apply {
        #[arg(short, long)]
        #[arg(help = "Template directory (default: data-dir/system)")]
        template: Option<PathBuf>,
    },

    #[command(about = "Generate system flake files")]
    GenerateFlake {
        #[arg(short, long)]
        #[arg(help = "Output directory (default: data-dir/system)")]
        output: Option<PathBuf>,

        #[arg(short, long)]
        #[arg(help = "Template directory to copy flake.nix, configuration.nix from")]
        template: Option<PathBuf>,

        #[arg(long)]
        #[arg(help = "Overwrite existing files")]
        force: bool,
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

    #[command(about = "LLM Model management")]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
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
pub enum AgentCommands {
    #[command(about = "List all agents")]
    List,

    #[command(about = "Create a new agent")]
    Create {
        name: String,
        #[arg(short, long)]
        forgejo_username: Option<String>,
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
        #[arg(long)]
        #[arg(help = "Checkpoint description")]
        desc: Option<String>,
    },

    #[command(about = "Rollback to a checkpoint")]
    Rollback { name: String, checkpoint_id: String },

    #[command(about = "List checkpoints for an agent")]
    ListCheckpoints { name: String },

    #[command(about = "Delete a checkpoint")]
    DeleteCheckpoint { checkpoint_id: String },
}

#[derive(Subcommand)]
pub enum CheckpointCommands {
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
pub enum GitCommands {
    #[command(about = "List Forgejo users")]
    Users,

    #[command(about = "Create a Forgejo user")]
    CreateUser {
        username: String,
        password: String,
        email: String,
    },

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
pub enum ConfigCommands {
    #[command(about = "Export config to YAML")]
    Export,

    #[command(about = "Import config from YAML")]
    Import,

    #[command(about = "Sync state to Forgejo and local tools/skills")]
    Sync {
        #[arg(long)]
        #[arg(help = "Delete orphaned resources not in state")]
        delete: bool,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommands {
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
pub enum ModelCommands {
    #[command(about = "List LLM models")]
    List,

    #[command(about = "Create a new model")]
    Create {
        name: String,
        #[arg(short, long)]
        provider: String,
        #[arg(short, long)]
        #[arg(help = "Real model name (e.g., gpt-4o, claude-3-opus)")]
        model_name: String,
    },

    #[command(about = "Delete a model")]
    Delete { id: String },
}

#[derive(Subcommand)]
pub enum SkillCommands {
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
pub enum ToolCommands {
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
    Authorize { slug: String, agent_name: String },

    #[command(about = "Revoke an agent's access to a tool")]
    Revoke { slug: String, agent_name: String },

    #[command(about = "Set an environment variable for a tool")]
    SetEnv {
        slug: String,
        key: String,
        value: String,
    },

    #[command(about = "List environment variables for a tool")]
    ListEnv { slug: String },

    #[command(about = "Delete an environment variable for a tool")]
    DeleteEnv { slug: String, key: String },

    #[command(about = "Invoke a tool")]
    Invoke {
        slug: String,
        #[arg(short, long)]
        #[arg(help = "JSON input")]
        input: String,
    },
}

pub fn get_data_dir(cli_data_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = cli_data_dir {
        return dir;
    }

    if let Ok(dir) = std::env::var("CEREBRATE_DATA_DIR") {
        return PathBuf::from(dir);
    }

    dirs::home_dir()
        .map(|h| h.join(".cerebrate"))
        .unwrap_or_else(|| PathBuf::from(".cerebrate"))
}

pub fn setup_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        assert_eq!(DEFAULT_PORT, 17531);
    }

    #[test]
    fn test_max_checkpoints() {
        assert_eq!(MAX_CHECKPOINTS_PER_AGENT, 10);
    }

    #[test]
    fn test_get_data_dir_from_arg() {
        let dir = get_data_dir(Some(PathBuf::from("/test/dir")));
        assert_eq!(dir, PathBuf::from("/test/dir"));
    }

    #[test]
    fn test_get_data_dir_from_env() {
        std::env::set_var("CEREBRATE_DATA_DIR", "/env/dir");
        let dir = get_data_dir(None);
        assert_eq!(dir, PathBuf::from("/env/dir"));
        std::env::remove_var("CEREBRATE_DATA_DIR");
    }

    #[test]
    fn test_get_data_dir_default() {
        std::env::remove_var("CEREBRATE_DATA_DIR");
        let dir = get_data_dir(None);
        assert!(dir.to_str().unwrap().contains(".cerebrate"));
    }

    #[test]
    fn test_cli_parse_serve() {
        let cli = Cli::try_parse_from(["cerebrate", "serve"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::try_parse_from(["cerebrate", "status"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_agent_list() {
        let cli = Cli::try_parse_from(["cerebrate", "agent", "list"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Agent { command } => match command {
                AgentCommands::List => (),
                _ => panic!("Wrong command"),
            },
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_cli_parse_agent_create() {
        let cli = Cli::try_parse_from(["cerebrate", "agent", "create", "my-agent"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_agent_create_with_forgejo() {
        let cli = Cli::try_parse_from(["cerebrate", "agent", "create", "my-agent", "-f", "user1"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_apply() {
        let cli = Cli::try_parse_from(["cerebrate", "apply"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_apply_with_template() {
        let cli = Cli::try_parse_from(["cerebrate", "apply", "--template", "/templates"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_config_export() {
        let cli = Cli::try_parse_from(["cerebrate", "config", "export"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_config_sync() {
        let cli = Cli::try_parse_from(["cerebrate", "config", "sync"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_config_sync_delete() {
        let cli = Cli::try_parse_from(["cerebrate", "config", "sync", "--delete"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_provider_list() {
        let cli = Cli::try_parse_from(["cerebrate", "provider", "list"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_provider_create() {
        let cli = Cli::try_parse_from([
            "cerebrate",
            "provider",
            "create",
            "OpenAI",
            "openai",
            "https://api.openai.com",
            "sk-test",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_tool_list() {
        let cli = Cli::try_parse_from(["cerebrate", "tool", "list"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_tool_clone() {
        let cli = Cli::try_parse_from([
            "cerebrate",
            "tool",
            "clone",
            "my-tool",
            "-r",
            "org/tool",
            "-a",
            "agent-1",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_skill_list() {
        let cli = Cli::try_parse_from(["cerebrate", "skill", "list"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_key_create() {
        let cli =
            Cli::try_parse_from(["cerebrate", "key", "create", "my-key", "-p", "provider-1"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_checkpoint_list() {
        let cli = Cli::try_parse_from(["cerebrate", "checkpoint", "list"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_checkpoint_list_with_agent() {
        let cli = Cli::try_parse_from(["cerebrate", "checkpoint", "list", "-a", "agent-1"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_with_data_dir() {
        let cli = Cli::try_parse_from(["cerebrate", "--data-dir", "/custom/dir", "serve"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert_eq!(cli.data_dir, Some(PathBuf::from("/custom/dir")));
    }

    #[test]
    fn test_cli_parse_tool_setenv() {
        let cli = Cli::try_parse_from([
            "cerebrate",
            "tool",
            "set-env",
            "my-tool",
            "API_KEY",
            "secret",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_tool_invoke() {
        let cli = Cli::try_parse_from([
            "cerebrate",
            "tool",
            "invoke",
            "my-tool",
            "-i",
            r#"{"query":"test"}"#,
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_git_users() {
        let cli = Cli::try_parse_from(["cerebrate", "git", "users"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_git_create_user() {
        let cli = Cli::try_parse_from([
            "cerebrate",
            "git",
            "create-user",
            "user1",
            "pass123",
            "user@example.com",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_git_create_org() {
        let cli = Cli::try_parse_from(["cerebrate", "git", "create-org", "myorg"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_git_create_repo() {
        let cli = Cli::try_parse_from(["cerebrate", "git", "create-repo", "myorg", "myrepo"]);
        assert!(cli.is_ok());
    }
}
