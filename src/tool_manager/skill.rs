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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_yaml_frontmatter_valid() {
        let content = r#"---
slug: my-skill
name: My Skill
version: "1.0"
description: A test skill
---
# Skill Content
"#;
        let result = parse_skill_yaml_frontmatter(content, "default").unwrap();
        assert_eq!(result.slug, "my-skill");
        assert_eq!(result.name, "My Skill");
        assert_eq!(result.description, "A test skill");
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_default_slug() {
        let content = r#"---
name: My Skill
version: "1.0"
slug: ""
description: ""
---
Content
"#;
        let result = parse_skill_yaml_frontmatter(content, "default-slug").unwrap();
        assert_eq!(result.slug, "default-slug");
        assert_eq!(result.name, "My Skill");
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_default_name() {
        let content = r#"---
slug: my-slug
version: "1.0"
name: ""
description: ""
---
Content
"#;
        let result = parse_skill_yaml_frontmatter(content, "default-name").unwrap();
        assert_eq!(result.slug, "my-slug");
        assert_eq!(result.name, "default-name");
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_missing_delimiters() {
        let content = "No frontmatter here";
        let result = parse_skill_yaml_frontmatter(content, "default");
        assert!(result.is_err());
        match result {
            Err(Error::Validation(msg)) => assert!(msg.contains("missing YAML frontmatter")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_one_delimiter() {
        let content = "---\nonly one delimiter";
        let result = parse_skill_yaml_frontmatter(content, "default");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_invalid_yaml() {
        let content = "---\n[invalid yaml\n---\nContent";
        let result = parse_skill_yaml_frontmatter(content, "default");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_with_description() {
        let content = r#"---
slug: test
name: Test
version: "1.0"
description: This is a test skill with a description
---
Content
"#;
        let result = parse_skill_yaml_frontmatter(content, "default").unwrap();
        assert_eq!(result.description, "This is a test skill with a description");
    }

    #[test]
    fn test_parse_skill_yaml_frontmatter_empty_description() {
        let content = r#"---
slug: test
name: Test
version: "1.0"
description: ""
---
Content
"#;
        let result = parse_skill_yaml_frontmatter(content, "default").unwrap();
        assert_eq!(result.description, "");
    }
}