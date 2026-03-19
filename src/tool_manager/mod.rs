mod git;
mod env;
mod invoke;
mod skill;

use crate::error::Result;
use crate::models::{InvokeToolResponse, Tool, ToolMetadata, SkillMetadata};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ToolManager {
    data_dir: PathBuf,
    forgejo_url: String,
    forgejo_token: String,
}

impl ToolManager {
    pub fn new(data_dir: PathBuf, forgejo_url: String, forgejo_token: String) -> Self {
        Self { data_dir, forgejo_url, forgejo_token }
    }

    pub fn tool_dir(&self, slug: &str) -> PathBuf {
        self.data_dir.join("tools").join(slug)
    }

    pub fn skill_dir(&self, slug: &str) -> PathBuf {
        self.data_dir.join("skills").join(slug)
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.data_dir.join("tools")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.data_dir.join("skills")
    }

    pub async fn ensure_directories(&self) -> Result<()> {
        tokio::fs::create_dir_all(self.data_dir.join("tools")).await?;
        tokio::fs::create_dir_all(self.data_dir.join("skills")).await?;
        Ok(())
    }

    fn git_ops(&self) -> git::GitOps {
        git::GitOps::new(self.data_dir.clone(), self.forgejo_url.clone(), self.forgejo_token.clone())
    }

    // Tool operations
    pub async fn clone_tool(&self, slug: &str, forgejo_repo: &str) -> Result<()> {
        let git = self.git_ops();
        let tool_dir = git.tool_dir(slug);
        git.clone(&tool_dir, forgejo_repo).await
    }

    pub async fn pull_tool(&self, slug: &str) -> Result<String> {
        let git = self.git_ops();
        let tool_dir = git.tool_dir(slug);
        git.pull(&tool_dir).await
    }

    pub fn parse_tool_md(&self, slug: &str) -> Result<ToolMetadata> {
        let tool_md = self.tool_dir(slug).join("TOOL.md");
        
        if !tool_md.exists() {
            return Err(crate::error::Error::NotFound(format!("TOOL.md not found for tool '{}'", slug)));
        }
        
        let content = std::fs::read_to_string(&tool_md)
            .map_err(|e| crate::error::Error::Io(e))?;
        
        parse_tool_yaml_frontmatter(&content, slug)
    }

    pub async fn get_git_commit(&self, slug: &str) -> Result<String> {
        let git = self.git_ops();
        let tool_dir = git.tool_dir(slug);
        git.get_commit(&tool_dir).await
    }

    pub async fn delete_tool(&self, slug: &str) -> Result<()> {
        let git = self.git_ops();
        let tool_dir = git.tool_dir(slug);
        git.delete(&tool_dir).await
    }

    // Environment variables
    pub async fn set_env(&self, slug: &str, key: &str, value: &str) -> Result<()> {
        env::set(&self.data_dir, slug, key, value).await
    }

    pub async fn list_env(&self, slug: &str) -> Result<Vec<String>> {
        env::list(&self.data_dir, slug).await
    }

    pub async fn delete_env(&self, slug: &str, key: &str) -> Result<()> {
        env::delete(&self.data_dir, slug, key).await
    }

    // Tool invocation
    pub async fn invoke_host_tool(&self, tool: &Tool, input: &serde_json::Value) -> Result<InvokeToolResponse> {
        let tool_dir = self.tool_dir(&tool.slug);
        let env_vars = env::load_all(&self.data_dir, &tool.slug);
        invoke::execute(&tool_dir, &tool.entrypoint, input, &env_vars).await
    }

    // Authorization
    pub fn check_authorization(&self, tool: &Tool, caller_agent: &str) -> bool {
        if tool.author_agent == caller_agent {
            return true;
        }
        tool.allowed_agents.contains(&caller_agent.to_string())
    }

    // Skill operations
    pub async fn clone_skill(&self, slug: &str, forgejo_repo: &str) -> Result<()> {
        skill::clone(&self.git_ops(), slug, forgejo_repo).await
    }

    pub async fn pull_skill(&self, slug: &str) -> Result<String> {
        skill::pull(&self.git_ops(), slug).await
    }

    pub fn parse_skill_md(&self, slug: &str) -> Result<SkillMetadata> {
        skill::parse_md(&self.git_ops(), slug)
    }

    pub async fn get_skill_git_commit(&self, slug: &str) -> Result<String> {
        skill::get_commit(&self.git_ops(), slug).await
    }

    pub async fn delete_skill(&self, slug: &str) -> Result<()> {
        skill::delete(&self.git_ops(), slug).await
    }
}

fn parse_tool_yaml_frontmatter(content: &str, default_slug: &str) -> Result<ToolMetadata> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(crate::error::Error::Validation("Invalid TOOL.md format: missing YAML frontmatter".into()));
    }
    
    let yaml_content = parts[1].trim();
    let mut metadata: ToolMetadata = serde_yaml::from_str(yaml_content)
        .map_err(|e| crate::error::Error::Validation(format!("Invalid YAML in TOOL.md: {}", e)))?;
    
    if metadata.slug.is_empty() {
        metadata.slug = default_slug.to_string();
    }
    if metadata.name.is_empty() {
        metadata.name = default_slug.to_string();
    }
    if metadata.entrypoint.is_empty() {
        metadata.entrypoint = "python main.py".to_string();
    }
    
    Ok(metadata)
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new(
            PathBuf::from("/var/lib/zerg-swarm"),
            "http://localhost:3000".to_string(),
            String::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_manager_new() {
        let tm = ToolManager::new(
            PathBuf::from("/data"),
            "http://localhost:3000".to_string(),
            "token".to_string(),
        );
        assert_eq!(tm.data_dir, PathBuf::from("/data"));
    }

    #[test]
    fn test_tool_manager_default() {
        let tm = ToolManager::default();
        assert_eq!(tm.data_dir, PathBuf::from("/var/lib/zerg-swarm"));
    }

    #[test]
    fn test_tool_dir() {
        let tm = ToolManager::default();
        let dir = tm.tool_dir("my-tool");
        assert!(dir.ends_with("tools/my-tool"));
    }

    #[test]
    fn test_skill_dir() {
        let tm = ToolManager::default();
        let dir = tm.skill_dir("my-skill");
        assert!(dir.ends_with("skills/my-skill"));
    }

    #[test]
    fn test_tools_dir() {
        let tm = ToolManager::default();
        let dir = tm.tools_dir();
        assert!(dir.ends_with("tools"));
    }

    #[test]
    fn test_skills_dir() {
        let tm = ToolManager::default();
        let dir = tm.skills_dir();
        assert!(dir.ends_with("skills"));
    }

    #[test]
    fn test_check_authorization_author() {
        let tm = ToolManager::default();
        let tool = Tool {
            slug: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "Test tool".to_string(),
            forgejo_repo: "tools/test".to_string(),
            git_commit: "abc".to_string(),
            entrypoint: "run.sh".to_string(),
            input_schema: None,
            output_schema: None,
            author_agent: "agent-1".to_string(),
            allowed_agents: vec!["agent-2".to_string()],
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        assert!(tm.check_authorization(&tool, "agent-1"));
    }

    #[test]
    fn test_check_authorization_allowed() {
        let tm = ToolManager::default();
        let tool = Tool {
            slug: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "Test tool".to_string(),
            forgejo_repo: "tools/test".to_string(),
            git_commit: "abc".to_string(),
            entrypoint: "run.sh".to_string(),
            input_schema: None,
            output_schema: None,
            author_agent: "agent-1".to_string(),
            allowed_agents: vec!["agent-2".to_string(), "agent-3".to_string()],
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        assert!(tm.check_authorization(&tool, "agent-2"));
        assert!(tm.check_authorization(&tool, "agent-3"));
    }

    #[test]
    fn test_check_authorization_denied() {
        let tm = ToolManager::default();
        let tool = Tool {
            slug: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "Test tool".to_string(),
            forgejo_repo: "tools/test".to_string(),
            git_commit: "abc".to_string(),
            entrypoint: "run.sh".to_string(),
            input_schema: None,
            output_schema: None,
            author_agent: "agent-1".to_string(),
            allowed_agents: vec!["agent-2".to_string()],
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        assert!(!tm.check_authorization(&tool, "agent-99"));
    }

    #[test]
    fn test_parse_tool_yaml_frontmatter_valid() {
        let content = r#"---
name: Test Tool
slug: test-tool
version: 1.0.0
description: A test tool
entrypoint: python main.py
---

# Tool Documentation
"#;
        let result = parse_tool_yaml_frontmatter(content, "default").unwrap();
        assert_eq!(result.name, "Test Tool");
        assert_eq!(result.slug, "test-tool");
        assert_eq!(result.version, "1.0.0");
    }

    #[test]
    fn test_parse_tool_yaml_frontmatter_defaults() {
        let content = r#"---
name: ""
slug: ""
version: 1.0.0
description: ""
entrypoint: ""
---
"#;
        let result = parse_tool_yaml_frontmatter(content, "my-tool").unwrap();
        assert_eq!(result.slug, "my-tool");
        assert_eq!(result.name, "my-tool");
        assert_eq!(result.entrypoint, "python main.py");
    }

    #[test]
    fn test_parse_tool_yaml_frontmatter_missing() {
        let content = "No frontmatter here";
        let result = parse_tool_yaml_frontmatter(content, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_manager_clone() {
        let tm = ToolManager::new(
            PathBuf::from("/data"),
            "http://localhost:3000".to_string(),
            "token".to_string(),
        );
        let cloned = tm.clone();
        assert_eq!(tm.data_dir, cloned.data_dir);
    }

    #[test]
    fn test_tool_manager_debug() {
        let tm = ToolManager::default();
        let debug = format!("{:?}", tm);
        assert!(debug.contains("ToolManager"));
    }

    #[test]
    fn test_tool_dir_nested_slug() {
        let tm = ToolManager::default();
        let dir = tm.tool_dir("org/tool");
        assert!(dir.ends_with("tools/org/tool"));
    }

    #[test]
    fn test_skill_dir_nested_slug() {
        let tm = ToolManager::default();
        let dir = tm.skill_dir("org/skill");
        assert!(dir.ends_with("skills/org/skill"));
    }

    #[test]
    fn test_check_authorization_empty_allowed_list() {
        let tm = ToolManager::default();
        let tool = Tool {
            slug: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "".to_string(),
            forgejo_repo: "".to_string(),
            git_commit: "".to_string(),
            entrypoint: "".to_string(),
            input_schema: None,
            output_schema: None,
            author_agent: "author".to_string(),
            allowed_agents: vec![],
            enabled: true,
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        
        assert!(tm.check_authorization(&tool, "author"));
        assert!(!tm.check_authorization(&tool, "other"));
    }

    #[tokio::test]
    async fn test_ensure_directories() {
        let temp = tempfile::tempdir().unwrap();
        let tm = ToolManager::new(
            temp.path().to_path_buf(),
            "http://localhost:3000".to_string(),
            "token".to_string(),
        );
        
        tm.ensure_directories().await.unwrap();
        assert!(temp.path().join("tools").exists());
        assert!(temp.path().join("skills").exists());
    }

    #[test]
    fn test_parse_tool_yaml_frontmatter_invalid_yaml() {
        let content = "---\n[invalid\n---\nContent";
        let result = parse_tool_yaml_frontmatter(content, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tool_yaml_frontmatter_one_delimiter() {
        let content = "---\nonly one delimiter";
        let result = parse_tool_yaml_frontmatter(content, "test");
        assert!(result.is_err());
    }
}