use crate::error::{Error, Result};
use crate::models::{InvokeSkillResponse, Skill, SkillMetadata, SkillType, SkillPermissions, NetworkPermissions, NetworkMode, FilesystemPermissions};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

const SKILLS_DIR: &str = "skills";
const SECRETS_DIR: &str = "secrets";

#[derive(Debug, Clone)]
pub struct SkillManager {
    data_dir: PathBuf,
    forgejo_url: String,
    forgejo_token: String,
}

impl SkillManager {
    pub fn new(data_dir: PathBuf, forgejo_url: String, forgejo_token: String) -> Self {
        Self { data_dir, forgejo_url, forgejo_token }
    }

    pub fn skill_dir(&self, name: &str) -> PathBuf {
        self.data_dir.join(SKILLS_DIR).join(name)
    }

    pub fn secrets_dir(&self, name: &str) -> PathBuf {
        self.data_dir.join(SECRETS_DIR).join(name)
    }

    pub async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(self.data_dir.join(SKILLS_DIR)).await?;
        fs::create_dir_all(self.data_dir.join(SECRETS_DIR)).await?;
        Ok(())
    }

    pub async fn clone_skill(&self, name: &str, forgejo_repo: &str) -> Result<()> {
        let skill_dir = self.skill_dir(name);
        
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir).await?;
        }
        
        let repo_url = format!("{}/{}.git", self.forgejo_url.trim_end_matches('/'), forgejo_repo);
        
        let status = tokio::process::Command::new("git")
            .args(["clone", "--depth", "1", &repo_url, &skill_dir.display().to_string()])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "true")
            .env("GIT_USERNAME", "oauth2")
            .env("GIT_PASSWORD", &self.forgejo_token)
            .status()
            .await
            .map_err(|e| Error::Io(e))?;
        
        if !status.success() {
            return Err(Error::TaskFailed(format!("Failed to clone skill from {}", repo_url)));
        }
        
        Ok(())
    }

    pub async fn pull_skill(&self, name: &str) -> Result<String> {
        let skill_dir = self.skill_dir(name);
        
        if !skill_dir.exists() {
            return Err(Error::NotFound(format!("Skill '{}' not found", name)));
        }
        
        let output = tokio::process::Command::new("git")
            .args(["pull", "--force"])
            .current_dir(&skill_dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "true")
            .env("GIT_USERNAME", "oauth2")
            .env("GIT_PASSWORD", &self.forgejo_token)
            .output()
            .await
            .map_err(|e| Error::Io(e))?;
        
        if !output.status.success() {
            return Err(Error::TaskFailed(format!("Failed to pull skill: {}", 
                String::from_utf8_lossy(&output.stderr))));
        }
        
        let commit_output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&skill_dir)
            .output()
            .await
            .map_err(|e| Error::Io(e))?;
        
        let commit = String::from_utf8_lossy(&commit_output.stdout).trim().to_string();
        Ok(commit)
    }

    pub fn parse_skill_md(&self, name: &str) -> Result<SkillMetadata> {
        let skill_md = self.skill_dir(name).join("SKILL.md");
        
        if !skill_md.exists() {
            return Err(Error::NotFound(format!("SKILL.md not found for skill '{}'", name)));
        }
        
        let content = std::fs::read_to_string(&skill_md)
            .map_err(|e| Error::Io(e))?;
        
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(Error::Validation("Invalid SKILL.md format: missing YAML frontmatter".into()));
        }
        
        let yaml_content = parts[1].trim();
        let mut metadata: SkillMetadata = serde_yaml::from_str(yaml_content)
            .map_err(|e| Error::Validation(format!("Invalid YAML in SKILL.md: {}", e)))?;
        
        if metadata.name.is_empty() {
            metadata.name = name.to_string();
        }
        if metadata.entrypoint.is_empty() {
            metadata.entrypoint = "python main.py".to_string();
        }
        
        Ok(metadata)
    }

    pub async fn get_git_commit(&self, name: &str) -> Result<String> {
        let skill_dir = self.skill_dir(name);
        
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&skill_dir)
            .output()
            .await
            .map_err(|e| Error::Io(e))?;
        
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(commit)
    }

    pub async fn delete_skill(&self, name: &str) -> Result<()> {
        let skill_dir = self.skill_dir(name);
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir).await?;
        }
        
        let secrets_dir = self.secrets_dir(name);
        if secrets_dir.exists() {
            fs::remove_dir_all(&secrets_dir).await?;
        }
        
        Ok(())
    }

    pub async fn set_secret(&self, name: &str, key: &str, value: &str) -> Result<()> {
        let dir = self.secrets_dir(name);
        fs::create_dir_all(&dir).await?;
        let path = dir.join(key);
        fs::write(&path, value).await?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await?;
        }
        Ok(())
    }

    pub async fn get_secret(&self, name: &str, key: &str) -> Result<Option<String>> {
        let path = self.secrets_dir(name).join(key);
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::Io(e)),
        }
    }

    pub async fn list_secrets(&self, name: &str) -> Result<Vec<String>> {
        let dir = self.secrets_dir(name);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&dir).await?;
        let mut keys = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    keys.push(name.to_string());
                }
            }
        }
        Ok(keys)
    }

    pub async fn delete_secret(&self, name: &str, key: &str) -> Result<()> {
        let path = self.secrets_dir(name).join(key);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        Ok(())
    }

    fn load_secrets_env(&self, name: &str) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();
        let secrets_dir = self.secrets_dir(name);
        
        if let Ok(entries) = std::fs::read_dir(&secrets_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    if let Some(key) = entry.file_name().to_str() {
                        if let Ok(value) = std::fs::read_to_string(entry.path()) {
                            let env_key = format!("SKILL_SECRET_{}", key.to_uppercase().replace("-", "_"));
                            env.insert(env_key, value);
                        }
                    }
                }
            }
        }
        env
    }

    pub async fn invoke_host_script(
        &self,
        skill: &Skill,
        input: &serde_json::Value,
    ) -> Result<InvokeSkillResponse> {
        let skill_dir = self.skill_dir(&skill.name);
        let input_json = serde_json::to_string(input)?;
        let secrets_env = self.load_secrets_env(&skill.name);
        
        let has_shell_nix = skill_dir.join("shell.nix").exists();
        
        let mut cmd = if has_shell_nix {
            self.build_nix_command(&skill_dir, &skill.entrypoint, &input_json, &secrets_env)
        } else {
            self.build_bwrap_command(&skill_dir, &skill.entrypoint, &input_json, &secrets_env, skill)
        };
        
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn().map_err(|e| Error::Io(e))?;
        
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
        
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        
        while !stdout_eof || !stderr_eof {
            tokio::select! {
                line = stdout_reader.next_line(), if !stdout_eof => {
                    match line {
                        Ok(Some(l)) => stdout_lines.push(l),
                        Ok(None) => stdout_eof = true,
                        Err(_) => stdout_eof = true,
                    }
                }
                line = stderr_reader.next_line(), if !stderr_eof => {
                    match line {
                        Ok(Some(l)) => stderr_lines.push(l),
                        Ok(None) => stderr_eof = true,
                        Err(_) => stderr_eof = true,
                    }
                }
            }
        }
        
        let status = child.wait().await.map_err(|e| Error::Io(e))?;
        
        if !status.success() {
            return Ok(InvokeSkillResponse {
                success: false,
                output: None,
                error: Some(format!("Script failed: {}", stderr_lines.join("\n"))),
            });
        }
        
        let output_str = stdout_lines.join("\n");
        
        match serde_json::from_str::<serde_json::Value>(&output_str) {
            Ok(json) => Ok(InvokeSkillResponse {
                success: true,
                output: Some(json),
                error: None,
            }),
            Err(_) => Ok(InvokeSkillResponse {
                success: true,
                output: Some(serde_json::json!({"raw": output_str})),
                error: None,
            }),
        }
    }

    fn build_bwrap_command(
        &self,
        skill_dir: &PathBuf,
        entrypoint: &str,
        input_json: &str,
        secrets_env: &std::collections::HashMap<String, String>,
        _skill: &Skill,
    ) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("bwrap");
        
        let skill_dir_str = skill_dir.display().to_string();
        
        cmd.args([
            "--ro-bind", "/nix/store", "/nix/store",
            "--ro-bind", &skill_dir_str, "/skill",
            "--dev", "/dev",
            "--proc", "/proc",
            "--tmpfs", "/tmp",
            "--die-with-parent",
            "--chdir", "/skill",
            "--unshare-ipc",
            "--unshare-pid",
        ]);
        
        for (key, value) in secrets_env {
            cmd.args(["--setenv", key, value]);
        }
        
        cmd.args(["--setenv", "SKILL_INPUT", input_json]);
        
        let shell_nix = skill_dir.join("shell.nix");
        if shell_nix.exists() {
            cmd.args([
                "--", "nix-shell", &skill_dir_str,
                "--run", &format!("{} '{}'", entrypoint, input_json.replace("'", "'\\''")),
            ]);
        } else {
            let parts: Vec<&str> = entrypoint.split_whitespace().collect();
            cmd.arg("--");
            for part in parts {
                cmd.arg(part.replace("{}", input_json));
            }
        }
        
        cmd
    }

    fn build_nix_command(
        &self,
        skill_dir: &PathBuf,
        entrypoint: &str,
        input_json: &str,
        secrets_env: &std::collections::HashMap<String, String>,
    ) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("nix-shell");
        
        cmd.arg(skill_dir)
           .arg("--run")
           .arg(&format!("{} '{}'", entrypoint, input_json.replace("'", "'\\''")));
        
        for (key, value) in secrets_env {
            cmd.env(key, value);
        }
        
        cmd.env("SKILL_INPUT", input_json);
        
        cmd
    }

    pub fn check_authorization(&self, skill: &Skill, caller_agent: &str) -> bool {
        if skill.author_agent == caller_agent {
            return true;
        }
        skill.allowed_agents.contains(&caller_agent.to_string())
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new(
            PathBuf::from("/var/lib/zerg-swarm"),
            "http://localhost:3000".to_string(),
            String::new(),
        )
    }
}