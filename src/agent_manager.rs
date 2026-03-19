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
        self.cleanup_removed_agents(&state.agents).await?;
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
            let flake_content = include_str!("../templates/flake.nix");
            tokio::fs::write(&flake_path, flake_content).await?;
        }

        let config_path = self.system_dir.join("configuration.nix");
        if !config_path.exists() {
            tracing::info!("Creating default configuration.nix");
            let config_content = include_str!("../templates/configuration.nix");
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

    async fn cleanup_removed_agents(&self, agents: &std::collections::HashMap<String, Agent>) -> Result<()> {
        let agents_dir = Path::new("/home/@agents");
        
        if !agents_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(agents_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // If this subvolume is not in the current agents list, delete it
            if !agents.contains_key(&name) {
                tracing::info!("Removing btrfs subvolume for deleted agent: {}", name);
                
                // Delete btrfs subvolume
                let status = tokio::process::Command::new("btrfs")
                    .args(["subvolume", "delete", entry.path().to_str().unwrap()])
                    .status()
                    .await;

                match status {
                    Ok(s) if s.success() => {
                        tracing::info!("Successfully removed subvolume for {}", name);
                    }
                    Ok(s) => {
                        tracing::warn!("Failed to remove subvolume for {} (exit code: {:?})", name, s.code());
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute btrfs subvolume delete: {}", e);
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
        println!("Running: nixos-rebuild switch --flake {}#zerg-swarm", flake_path);

        let mut child = tokio::process::Command::new("systemd-run")
            .args([
                "--scope",
                "--quiet",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Agent, Defaults, State};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_agent() -> Agent {
        Agent {
            enabled: true,
            container_ip: "10.0.0.2".to_string(),
            host_ip: "10.0.0.1".to_string(),
            forgejo_username: None,
            internal_token: "test-token".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        }
    }

    #[test]
    fn test_agent_manager_new() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        assert_eq!(manager.system_dir, dir.path());
        assert_eq!(manager.generated_dir, dir.path().join("generated"));
        assert!(manager.template_dir.is_none());
    }

    #[test]
    fn test_agent_manager_with_template() {
        let dir = tempdir().unwrap();
        let template_dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path()).with_template(template_dir.path());
        assert_eq!(manager.template_dir, Some(template_dir.path().to_path_buf()));
    }

    #[test]
    fn test_generate_containers_nix_empty() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let state = State::default();
        let output = manager.generate_containers_nix(&state);
        assert!(output.contains("No containers configured"));
    }

    #[test]
    fn test_generate_containers_nix_with_agents() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut state = State::default();
        state.defaults = Defaults {
            port: 8080,
            ..Default::default()
        };
        let mut agent = create_test_agent();
        agent.enabled = true;
        agent.host_ip = "10.0.0.1".to_string();
        agent.container_ip = "10.0.0.2".to_string();
        agent.internal_token = "test-token".to_string();
        state.agents.insert("agent1".to_string(), agent);
        let output = manager.generate_containers_nix(&state);
        assert!(output.contains("containers.agent1"));
        assert!(output.contains("10.0.0.1"));
        assert!(output.contains("10.0.0.2"));
        assert!(output.contains("test-token"));
        assert!(output.contains("autoStart = true"));
    }

    #[test]
    fn test_generate_containers_nix_disabled_agent() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut state = State::default();
        let mut agent = create_test_agent();
        agent.enabled = false;
        agent.host_ip = "10.0.0.1".to_string();
        agent.container_ip = "10.0.0.2".to_string();
        agent.internal_token = "token".to_string();
        state.agents.insert("disabled-agent".to_string(), agent);
        let output = manager.generate_containers_nix(&state);
        assert!(output.contains("autoStart = false"));
    }

    #[test]
    fn test_generate_containers_nix_multiple_agents() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut state = State::default();
        state.defaults = Defaults {
            port: 9000,
            ..Default::default()
        };
        for i in 0..3 {
            let mut agent = create_test_agent();
            agent.enabled = true;
            agent.host_ip = format!("10.0.{}.1", i);
            agent.container_ip = format!("10.0.{}.2", i);
            agent.internal_token = format!("token-{}", i);
            state.agents.insert(format!("agent{}", i), agent);
        }
        let output = manager.generate_containers_nix(&state);
        assert!(output.contains("containers.agent0"));
        assert!(output.contains("containers.agent1"));
        assert!(output.contains("containers.agent2"));
        assert!(output.contains("9000"));
    }

    #[test]
    fn test_generate_filesystem_nix_empty() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let agents = HashMap::new();
        let output = manager.generate_filesystem_nix(&agents, "/dev/sda1");
        assert!(output.contains("No agent filesystems"));
    }

    #[test]
    fn test_generate_filesystem_nix_with_agents() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), create_test_agent());
        agents.insert("agent2".to_string(), create_test_agent());
        let output = manager.generate_filesystem_nix(&agents, "/dev/nvme0n1");
        assert!(output.contains("agent1"));
        assert!(output.contains("agent2"));
        assert!(output.contains("/dev/nvme0n1"));
        assert!(output.contains("btrfs"));
        assert!(output.contains("subvol=@agents/agent1"));
        assert!(output.contains("compress=zstd"));
    }

    #[test]
    fn test_generate_filesystem_nix_device_path() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut agents = HashMap::new();
        agents.insert("myagent".to_string(), create_test_agent());
        let output = manager.generate_filesystem_nix(&agents, "/dev/mapper/root");
        assert!(output.contains("/dev/mapper/root"));
    }

    #[tokio::test]
    async fn test_ensure_directories() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        manager.ensure_directories().await.unwrap();
        assert!(dir.path().join("generated").exists());
    }

    #[tokio::test]
    async fn test_ensure_directories_existing() {
        let dir = tempdir().unwrap();
        tokio::fs::create_dir_all(dir.path().join("generated")).await.unwrap();
        let manager = AgentManager::new(dir.path());
        manager.ensure_directories().await.unwrap();
        assert!(dir.path().join("generated").exists());
    }

    #[tokio::test]
    async fn test_write_containers_nix() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        manager.ensure_directories().await.unwrap();
        let state = State::default();
        manager.write_containers_nix(&state).await.unwrap();
        let content = tokio::fs::read_to_string(dir.path().join("generated/container.nix")).await.unwrap();
        assert!(content.contains("No containers"));
    }

    #[tokio::test]
    async fn test_write_filesystem_nix() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        manager.ensure_directories().await.unwrap();
        let agents = HashMap::new();
        manager.write_filesystem_nix(&agents, "/dev/sda1").await.unwrap();
        let content = tokio::fs::read_to_string(dir.path().join("generated/filesystem.nix")).await.unwrap();
        assert!(content.contains("No agent filesystems"));
    }

    #[test]
    fn test_generated_nix_format() {
        let dir = tempdir().unwrap();
        let manager = AgentManager::new(dir.path());
        let mut state = State::default();
        state.defaults = Defaults {
            port: 8080,
            ..Default::default()
        };
        let mut agent = create_test_agent();
        agent.enabled = true;
        agent.host_ip = "192.168.1.1".to_string();
        agent.container_ip = "192.168.1.2".to_string();
        agent.internal_token = "secret-token-123".to_string();
        state.agents.insert("testagent".to_string(), agent);
        let output = manager.generate_containers_nix(&state);
        assert!(output.contains("openzerg.nixosModules.default"));
        assert!(output.contains("services.openzerg"));
        assert!(output.contains("ws://192.168.1.1:8080"));
    }
}