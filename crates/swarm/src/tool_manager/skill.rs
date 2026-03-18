use crate::error::{Error, Result};
use crate::models::SkillMetadata;
use super::git::GitOps;

pub async fn clone(git: &GitOps, slug: &str, forgejo_repo: &str) -> Result<()> {
    let skill_dir = git.skill_dir(slug);
    git.clone(&skill_dir, forgejo_repo).await
}

pub async fn pull(git: &GitOps, slug: &str) -> Result<String> {
    let skill_dir = git.skill_dir(slug);
    git.pull(&skill_dir).await
}

pub fn parse_md(git: &GitOps, slug: &str) -> Result<SkillMetadata> {
    let skill_md = git.skill_dir(slug).join("SKILL.md");
    
    if !skill_md.exists() {
        return Err(Error::NotFound(format!("SKILL.md not found for skill '{}'", slug)));
    }
    
    let content = std::fs::read_to_string(&skill_md)
        .map_err(|e| Error::Io(e))?;
    
    parse_skill_yaml_frontmatter(&content, slug)
}

pub async fn get_commit(git: &GitOps, slug: &str) -> Result<String> {
    let skill_dir = git.skill_dir(slug);
    git.get_commit(&skill_dir).await
}

pub async fn delete(git: &GitOps, slug: &str) -> Result<()> {
    let skill_dir = git.skill_dir(slug);
    git.delete(&skill_dir).await
}

fn parse_skill_yaml_frontmatter(content: &str, default_slug: &str) -> Result<SkillMetadata> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(Error::Validation("Invalid SKILL.md format: missing YAML frontmatter".into()));
    }
    
    let yaml_content = parts[1].trim();
    let mut metadata: SkillMetadata = serde_yaml::from_str(yaml_content)
        .map_err(|e| Error::Validation(format!("Invalid YAML in SKILL.md: {}", e)))?;
    
    if metadata.slug.is_empty() {
        metadata.slug = default_slug.to_string();
    }
    if metadata.name.is_empty() {
        metadata.name = default_slug.to_string();
    }
    
    Ok(metadata)
}