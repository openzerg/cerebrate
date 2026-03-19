use crate::forgejo;
use crate::models::State;
use crate::tool_manager::ToolManager;
use crate::Result;
use std::path::Path;

pub struct SyncResult {
    pub users_created: Vec<String>,
    pub users_deleted: Vec<String>,
    pub tools_cloned: Vec<String>,
    pub skills_cloned: Vec<String>,
}

impl SyncResult {
    pub fn new() -> Self {
        Self {
            users_created: Vec::new(),
            users_deleted: Vec::new(),
            tools_cloned: Vec::new(),
            skills_cloned: Vec::new(),
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.users_created.is_empty()
            || !self.users_deleted.is_empty()
            || !self.tools_cloned.is_empty()
            || !self.skills_cloned.is_empty()
    }

    pub fn print_summary(&self) {
        if !self.has_changes() {
            println!("Already in sync, no changes needed.");
            return;
        }

        println!("Sync completed:");
        if !self.users_created.is_empty() {
            println!("  Users created: {}", self.users_created.join(", "));
        }
        if !self.users_deleted.is_empty() {
            println!("  Users deleted: {}", self.users_deleted.join(", "));
        }
        if !self.tools_cloned.is_empty() {
            println!("  Tools cloned: {}", self.tools_cloned.join(", "));
        }
        if !self.skills_cloned.is_empty() {
            println!("  Skills cloned: {}", self.skills_cloned.join(", "));
        }
    }
}

pub async fn sync_all(
    state: &State,
    data_dir: &Path,
    delete_orphans: bool,
) -> Result<SyncResult> {
    let mut result = SyncResult::new();
    
    let forgejo_url = &state.defaults.forgejo_url;
    let forgejo_token = &state.defaults.forgejo_token;
    
    if forgejo_token.is_empty() {
        println!("Warning: forgejo_token not configured, skipping Forgejo sync");
        return Ok(result);
    }

    sync_forgejo_users(state, forgejo_url, forgejo_token, delete_orphans, &mut result).await?;
    
    let tool_manager = ToolManager::new(
        data_dir.to_path_buf(),
        forgejo_url.clone(),
        forgejo_token.clone(),
    );
    sync_tools(state, &tool_manager, &mut result).await?;
    sync_skills(state, &tool_manager, &mut result).await?;

    Ok(result)
}

pub async fn sync_forgejo_users(
    state: &State,
    forgejo_url: &str,
    forgejo_token: &str,
    delete_orphans: bool,
    result: &mut SyncResult,
) -> Result<()> {
    println!("Syncing Forgejo users...");
    
    let existing_users = forgejo::user::list_users(forgejo_url, forgejo_token).await?;
    let existing_usernames: std::collections::HashSet<String> = 
        existing_users.iter().map(|u| u.login.clone()).collect();
    
    let desired_usernames: std::collections::HashSet<String> = 
        state.forgejo_users.keys().cloned().collect();
    
    for (username, user) in &state.forgejo_users {
        if !existing_usernames.contains(username) {
            println!("  Creating user: {}", username);
            forgejo::user::create_user(forgejo_url, forgejo_token, username, &user.password, &user.email).await?;
            result.users_created.push(username.clone());
        }
    }
    
    if delete_orphans {
        for existing in existing_usernames {
            if !desired_usernames.contains(&existing) && existing != "root" {
                println!("  Deleting user: {}", existing);
                forgejo::user::delete_user(forgejo_url, forgejo_token, &existing).await?;
                result.users_deleted.push(existing);
            }
        }
    }
    
    Ok(())
}

pub async fn sync_tools(
    state: &State,
    tool_manager: &ToolManager,
    result: &mut SyncResult,
) -> Result<()> {
    println!("Syncing tools...");
    
    for (slug, tool) in &state.tools {
        let tool_path = tool_manager.tools_dir().join(slug);
        if !tool_path.exists() {
            println!("  Cloning tool: {} from {}", slug, tool.forgejo_repo);
            if let Err(e) = tool_manager.clone_tool(slug, &tool.forgejo_repo).await {
                println!("    Warning: Failed to clone tool: {}", e);
            } else {
                result.tools_cloned.push(slug.clone());
            }
        }
    }
    
    Ok(())
}

pub async fn sync_skills(
    state: &State,
    tool_manager: &ToolManager,
    result: &mut SyncResult,
) -> Result<()> {
    println!("Syncing skills...");
    
    for (slug, skill) in &state.skills {
        let skill_path = tool_manager.skills_dir().join(slug);
        if !skill_path.exists() {
            println!("  Cloning skill: {} from {}", slug, skill.forgejo_repo);
            if let Err(e) = tool_manager.clone_skill(slug, &skill.forgejo_repo).await {
                println!("    Warning: Failed to clone skill: {}", e);
            } else {
                result.skills_cloned.push(slug.clone());
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_new() {
        let result = SyncResult::new();
        assert!(result.users_created.is_empty());
        assert!(result.users_deleted.is_empty());
        assert!(result.tools_cloned.is_empty());
        assert!(result.skills_cloned.is_empty());
    }

    #[test]
    fn test_sync_result_has_changes_empty() {
        let result = SyncResult::new();
        assert!(!result.has_changes());
    }

    #[test]
    fn test_sync_result_has_changes_with_users() {
        let mut result = SyncResult::new();
        result.users_created.push("user1".to_string());
        assert!(result.has_changes());
    }

    #[test]
    fn test_sync_result_has_changes_with_tools() {
        let mut result = SyncResult::new();
        result.tools_cloned.push("tool1".to_string());
        assert!(result.has_changes());
    }

    #[test]
    fn test_sync_result_has_changes_with_skills() {
        let mut result = SyncResult::new();
        result.skills_cloned.push("skill1".to_string());
        assert!(result.has_changes());
    }

    #[test]
    fn test_sync_result_has_changes_with_deleted() {
        let mut result = SyncResult::new();
        result.users_deleted.push("user1".to_string());
        assert!(result.has_changes());
    }

    #[test]
    fn test_sync_result_print_summary_no_changes() {
        let result = SyncResult::new();
        result.print_summary();
    }

    #[test]
    fn test_sync_result_print_summary_with_users_created() {
        let mut result = SyncResult::new();
        result.users_created.push("user1".to_string());
        result.users_created.push("user2".to_string());
        result.print_summary();
    }

    #[test]
    fn test_sync_result_print_summary_with_all_changes() {
        let mut result = SyncResult::new();
        result.users_created.push("user1".to_string());
        result.users_deleted.push("olduser".to_string());
        result.tools_cloned.push("tool1".to_string());
        result.skills_cloned.push("skill1".to_string());
        result.print_summary();
    }

    #[test]
    fn test_sync_result_multiple_items() {
        let mut result = SyncResult::new();
        result.users_created.push("a".to_string());
        result.users_created.push("b".to_string());
        result.users_created.push("c".to_string());
        assert_eq!(result.users_created.len(), 3);
        assert!(result.has_changes());
    }

    #[tokio::test]
    async fn test_sync_all_empty_token() {
        let temp = tempfile::tempdir().unwrap();
        let state = State::default();
        let result = sync_all(&state, temp.path(), false).await.unwrap();
        assert!(!result.has_changes());
    }
}