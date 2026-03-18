use crate::error::{Error, Result};
use crate::models::{InvokeSkillRequest, InvokeSkillResponse, Skill, SkillType};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

const SKILLS_DIR: &str = "skills";
const SECRETS_DIR: &str = "secrets";

#[derive(Debug, Clone)]
pub struct SkillManager {
    data_dir: PathBuf,
}

impl SkillManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn skill_dir(&self, skill_id: &str) -> PathBuf {
        self.data_dir.join(SKILLS_DIR).join(skill_id)
    }

    pub fn secrets_dir(&self, skill_id: &str) -> PathBuf {
        self.data_dir.join(SECRETS_DIR).join(skill_id)
    }

    pub async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(self.data_dir.join(SKILLS_DIR)).await?;
        fs::create_dir_all(self.data_dir.join(SECRETS_DIR)).await?;
        Ok(())
    }

    pub async fn write_skill_file(&self, skill_id: &str, filename: &str, content: &str) -> Result<()> {
        let dir = self.skill_dir(skill_id);
        fs::create_dir_all(&dir).await?;
        fs::write(dir.join(filename), content).await?;
        Ok(())
    }

    pub async fn read_skill_file(&self, skill_id: &str, filename: &str) -> Result<String> {
        let path = self.skill_dir(skill_id).join(filename);
        fs::read_to_string(&path).await.map_err(|e| Error::Io(e))
    }

    pub async fn list_skill_files(&self, skill_id: &str) -> Result<Vec<String>> {
        let dir = self.skill_dir(skill_id);
        let mut entries = fs::read_dir(&dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }
        Ok(files)
    }

    pub async fn delete_skill_files(&self, skill_id: &str) -> Result<()> {
        let dir = self.skill_dir(skill_id);
        if dir.exists() {
            fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    pub async fn set_secret(&self, skill_id: &str, key: &str, value: &str) -> Result<()> {
        let dir = self.secrets_dir(skill_id);
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

    pub async fn get_secret(&self, skill_id: &str, key: &str) -> Result<Option<String>> {
        let path = self.secrets_dir(skill_id).join(key);
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::Io(e)),
        }
    }

    pub async fn list_secrets(&self, skill_id: &str) -> Result<Vec<String>> {
        let dir = self.secrets_dir(skill_id);
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

    pub async fn delete_secret(&self, skill_id: &str, key: &str) -> Result<()> {
        let path = self.secrets_dir(skill_id).join(key);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        Ok(())
    }

    pub async fn delete_all_secrets(&self, skill_id: &str) -> Result<()> {
        let dir = self.secrets_dir(skill_id);
        if dir.exists() {
            fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    fn load_secrets_env(&self, skill_id: &str) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();
        let secrets_dir = self.secrets_dir(skill_id);
        
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
        let skill_dir = self.skill_dir(&skill.id);
        let entrypoint = &skill.entrypoint;
        
        let input_json = serde_json::to_string(input)?;
        
        let secrets_env = self.load_secrets_env(&skill.id);
        
        let mut cmd = tokio::process::Command::new("nix-shell");
        cmd.arg("--pure")
           .arg(&skill_dir)
           .arg("--run")
           .arg(&format!("{} '{}' '{}'", entrypoint, skill.id, input_json.replace("'", "'\\''")));
        
        for (key, value) in &secrets_env {
            cmd.env(key, value);
        }
        
        cmd.env("SKILL_ID", &skill.id);
        cmd.env("SKILL_INPUT", &input_json);
        
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
                        Ok(Some(l)) => {
                            stdout_lines.push(l);
                        }
                        Ok(None) => stdout_eof = true,
                        Err(_) => stdout_eof = true,
                    }
                }
                line = stderr_reader.next_line(), if !stderr_eof => {
                    match line {
                        Ok(Some(l)) => {
                            stderr_lines.push(l);
                        }
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

    pub async fn read_all_skill_files(&self, skill_id: &str) -> Result<Vec<(String, String)>> {
        let dir = self.skill_dir(skill_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = fs::read_dir(&dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if entry.file_type().await?.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        files.push((name.to_string(), content));
                    }
                }
            }
        }
        Ok(files)
    }

    pub fn check_authorization(&self, skill: &Skill, caller_agent: &str) -> bool {
        if skill.owner_agent == caller_agent {
            return true;
        }
        skill.allowed_agents.contains(&caller_agent.to_string())
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new(PathBuf::from("/var/lib/zerg-swarm"))
    }
}