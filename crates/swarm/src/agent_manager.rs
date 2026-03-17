use crate::models::{Agent, Defaults, State};
use crate::{Error, Result};
use std::path::Path;

#[derive(Clone)]
pub struct AgentManager {
    system_dir: std::path::PathBuf,
    generated_dir: std::path::PathBuf,
    template_dir: Option<std::path::PathBuf>,
}

impl AgentManager {
    pub fn new(system_dir: &Path) -> Self {
        Self {
            system_dir: system_dir.to_path_buf(),
            generated_dir: system_dir.join("generated"),
            template_dir: None,
        }
    }

    pub fn with_template(mut self, template_dir: &Path) -> Self {
        self.template_dir = Some(template_dir.to_path_buf());
        self
    }

    pub async fn apply(&self, state: &State, btrfs_device: &str) -> Result<()> {
        self.ensure_directories().await?;
        self.ensure_template_files().await?;
        self.ensure_btrfs_subvolumes(&state.agents).await?;
        self.write_containers_nix(state).await?;
        self.write_filesystem_nix(&state.agents, btrfs_device).await?;
        self.nixos_rebuild_switch().await?;
        Ok(())
    }

    async fn ensure_directories(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.generated_dir).await?;
        Ok(())
    }

    async fn ensure_template_files(&self) -> Result<()> {
        let flake_path = self.system_dir.join("flake.nix");
        if !flake_path.exists() {
            tracing::info!("Creating default flake.nix");
            let flake_content = include_str!("../../../templates/flake.nix");
            tokio::fs::write(&flake_path, flake_content).await?;
        }

        let config_path = self.system_dir.join("configuration.nix");
        if !config_path.exists() {
            tracing::info!("Creating default configuration.nix");
            let config_content = include_str!("../../../templates/configuration.nix");
            tokio::fs::write(&config_path, config_content).await?;
        }

        let hardware_path = self.system_dir.join("hardware-configuration.nix");
        if !hardware_path.exists() {
            tracing::warn!("hardware-configuration.nix not found, please run 'nixos-generate-config' or copy from /etc/nixos/hardware-configuration.nix");
        }

        Ok(())
    }

    async fn ensure_btrfs_subvolumes(&self, agents: &std::collections::HashMap<String, Agent>) -> Result<()> {
        let agents_dir = Path::new("/home/@agents");
        
        // Create @agents parent directory if it doesn't exist
        if !agents_dir.exists() {
            tracing::info!("Creating @agents directory");
            tokio::fs::create_dir_all(agents_dir).await?;
        }
        
        for agent_name in agents.keys() {
            let agent_path = agents_dir.join(agent_name);
            if !agent_path.exists() {
                tracing::info!("Creating btrfs subvolume for agent: {}", agent_name);
                
                // Use btrfs from PATH (service runs as root)
                let status = tokio::process::Command::new("btrfs")
                    .args(["subvolume", "create", agent_path.to_str().unwrap()])
                    .status()
                    .await;

                match status {
                    Ok(s) if s.success() => {
                        tracing::info!("Successfully created subvolume for {}", agent_name);
                    }
                    Ok(s) => {
                        tracing::warn!("Failed to create subvolume for {} (exit code: {:?})", agent_name, s.code());
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute btrfs command: {}", e);
                        return Err(Error::Io(e));
                    }
                }
            }
        }
        Ok(())
    }

    async fn write_containers_nix(&self, state: &State) -> Result<()> {
        let content = self.generate_containers_nix(state);
        tokio::fs::write(self.generated_dir.join("container.nix"), content).await?;
        Ok(())
    }

    async fn write_filesystem_nix(&self, agents: &std::collections::HashMap<String, Agent>, btrfs_device: &str) -> Result<()> {
        let content = self.generate_filesystem_nix(agents, btrfs_device);
        tokio::fs::write(self.generated_dir.join("filesystem.nix"), content).await?;
        Ok(())
    }

    async fn copy_template_files(&self) -> Result<()> {
        if let Some(template_dir) = &self.template_dir {
            // Copy flake.nix, configuration.nix, hardware-configuration.nix
            for file in ["flake.nix", "configuration.nix", "hardware-configuration.nix"] {
                let src = template_dir.join(file);
                let dst = self.system_dir.join(file);
                if src.exists() {
                    tokio::fs::copy(&src, &dst).await?;
                }
            }
        }
        Ok(())
    }

    async fn nixos_rebuild_switch(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        
        let flake_path = self.system_dir.to_string_lossy();
        println!("Running: sudo nixos-rebuild switch --flake {}#zerg-swarm", flake_path);

        let mut child = tokio::process::Command::new("sudo")
            .args([
                "nixos-rebuild",
                "switch",
                "--flake",
                &format!("{}#zerg-swarm", flake_path),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| Error::Io(e))?;

        let stdout = child.stdout.take().ok_or_else(|| Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to capture stdout",
        )))?;
        let stderr = child.stderr.take().ok_or_else(|| Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to capture stderr",
        )))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_eof = false;
        let mut stderr_eof = false;

        while !stdout_eof || !stderr_eof {
            tokio::select! {
                line = stdout_reader.next_line(), if !stdout_eof => {
                    match line {
                        Ok(Some(line)) => println!("{}", line),
                        Ok(None) => stdout_eof = true,
                        Err(e) => {
                            eprintln!("Error reading stdout: {}", e);
                            stdout_eof = true;
                        }
                    }
                }
                line = stderr_reader.next_line(), if !stderr_eof => {
                    match line {
                        Ok(Some(line)) => eprintln!("{}", line),
                        Ok(None) => stderr_eof = true,
                        Err(e) => {
                            eprintln!("Error reading stderr: {}", e);
                            stderr_eof = true;
                        }
                    }
                }
            }
        }

        let status = child.wait().await.map_err(|e| Error::Io(e))?;

        if !status.success() {
            return Err(Error::Config("nixos-rebuild failed".to_string()));
        }

        Ok(())
    }

    fn generate_containers_nix(&self, state: &State) -> String {
        if state.agents.is_empty() {
            return r#"# Auto-generated by Zerg Swarm
# No containers configured

{ config, pkgs, lib, openzerg, ... }:

{
  # No containers
}
"#.to_string();
        }

        let mut containers = String::new();

        for (name, agent) in &state.agents {
            let container = format!(
                r#"  containers.{name} = {{
    autoStart = {auto_start};
    ephemeral = false;
    privateNetwork = true;
    hostAddress = "{host_ip}";
    localAddress = "{container_ip}";
    
    bindMounts = {{
      "/workspace" = {{
        hostPath = "/var/lib/agents/{name}";
        isReadOnly = false;
      }};
    }};
    
    config = {{ config, pkgs, lib, ... }}: {{
      imports = [
        openzerg.nixosModules.default
      ];
      
      services.openzerg = {{
        enable = true;
        package = openzerg.packages.${{pkgs.stdenv.hostPlatform.system}}.openzerg;
        agentName = "{name}";
        managerUrl = "ws://{host_ip}:{ws_port}";
        internalToken = "{token}";
        workspace = "/workspace";
      }};
      
      networking.firewall.allowedTCPPorts = [ 8080 ];
      
      system.stateVersion = "25.11";
    }};
  }};
"#,
                name = name,
                auto_start = if agent.enabled { "true" } else { "false" },
                host_ip = agent.host_ip,
                container_ip = agent.container_ip,
                ws_port = state.defaults.port,
                token = agent.internal_token,
            );
            containers.push_str(&container);
        }

        format!(
            r#"# Auto-generated by Zerg Swarm
# DO NOT EDIT MANUALLY

{{ config, pkgs, lib, openzerg, ... }}:

{{
{containers}}}
"#
        )
    }

    fn generate_filesystem_nix(&self, agents: &std::collections::HashMap<String, Agent>, btrfs_device: &str) -> String {
        if agents.is_empty() {
            return r#"# Auto-generated by Zerg Swarm
# No agent filesystems

{ config, ... }:

{
  # No agent filesystems
}
"#.to_string();
        }

        let mut filesystems = String::new();

        for name in agents.keys() {
            let fs = format!(
                r#"  fileSystems."/var/lib/agents/{name}" = {{
    device = "{device}";
    fsType = "btrfs";
    options = [ "subvol=@agents/{name}" "compress=zstd" "noatime" ];
  }};

"#,
                name = name,
                device = btrfs_device,
            );
            filesystems.push_str(&fs);
        }

        format!(
            r#"# Auto-generated by Zerg Swarm
# DO NOT EDIT MANUALLY

{{ config, ... }}:

{{
{filesystems}}}
"#
        )
    }
}