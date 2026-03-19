use crate::forgejo::ForgejoClient;
use crate::{Result, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub description: String,
    pub private: bool,
    pub fork: bool,
    pub html_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub stars_count: i64,
    pub watchers_count: i64,
    pub forks_count: i64,
    pub open_issues_count: i64,
    pub default_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub id: i64,
    pub login: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
struct CreateRepoRequest {
    name: String,
    description: String,
    private: bool,
    auto_init: bool,
}

#[derive(Debug, Serialize)]
struct TransferRepoRequest {
    new_owner: String,
}

#[derive(Debug, Serialize)]
struct UpdateRepoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    private: Option<bool>,
}

pub async fn list_repos(forgejo_url: &str, forgejo_token: &str, owner: Option<&str>) -> Result<Vec<Repository>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let url = match owner {
        Some(owner) => client.url(&format!("/users/{}/repos", owner)),
        None => client.url("/user/repos"),
    };
    
    let response = client
        .http()
        .get(&url)
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to list repos: {} - {}", status, body)));
    }
    
    let repos: Vec<Repository> = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse repos: {}", e)))?;
    
    Ok(repos)
}

pub async fn get_repo(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str) -> Result<Option<Repository>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url(&format!("/repos/{}/{}", owner, repo)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to get repo: {} - {}", status, body)));
    }
    
    let repo: Repository = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse repo: {}", e)))?;
    
    Ok(Some(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_serialization() {
        let repo = Repository {
            id: 1,
            name: "test-repo".to_string(),
            full_name: "owner/test-repo".to_string(),
            owner: Owner {
                id: 1,
                login: "owner".to_string(),
                full_name: "Owner Name".to_string(),
                email: "owner@example.com".to_string(),
                avatar_url: "https://example.com/avatar.png".to_string(),
            },
            description: "A test repo".to_string(),
            private: true,
            fork: false,
            html_url: "https://example.com/owner/test-repo".to_string(),
            ssh_url: "git@example.com:owner/test-repo.git".to_string(),
            clone_url: "https://example.com/owner/test-repo.git".to_string(),
            stars_count: 100,
            watchers_count: 50,
            forks_count: 10,
            open_issues_count: 5,
            default_branch: "main".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
        };
        
        let json = serde_json::to_string(&repo).unwrap();
        assert!(json.contains("test-repo"));
        assert!(json.contains("owner"));
        assert!(json.contains("A test repo"));
    }

    #[test]
    fn test_repository_deserialization() {
        let json = r#"{
            "id": 42,
            "name": "my-repo",
            "full_name": "user/my-repo",
            "owner": {
                "id": 1,
                "login": "user",
                "full_name": "User Name",
                "email": "user@example.com",
                "avatar_url": "https://example.com/avatar.png"
            },
            "description": "Test description",
            "private": false,
            "fork": true,
            "html_url": "https://example.com/user/my-repo",
            "ssh_url": "git@example.com:user/my-repo.git",
            "clone_url": "https://example.com/user/my-repo.git",
            "stars_count": 0,
            "watchers_count": 0,
            "forks_count": 0,
            "open_issues_count": 0,
            "default_branch": "main",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        
        let repo: Repository = serde_json::from_str(json).unwrap();
        assert_eq!(repo.id, 42);
        assert_eq!(repo.name, "my-repo");
        assert_eq!(repo.owner.login, "user");
        assert!(!repo.private);
        assert!(repo.fork);
    }

    #[test]
    fn test_owner_serialization() {
        let owner = Owner {
            id: 123,
            login: "testowner".to_string(),
            full_name: "Test Owner".to_string(),
            email: "owner@test.com".to_string(),
            avatar_url: "https://example.com/avatar.png".to_string(),
        };
        
        let json = serde_json::to_string(&owner).unwrap();
        assert!(json.contains("testowner"));
        assert!(json.contains("Test Owner"));
    }

    #[test]
    fn test_create_repo_request_serialization() {
        let req = CreateRepoRequest {
            name: "new-repo".to_string(),
            description: "New repository".to_string(),
            private: true,
            auto_init: false,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("new-repo"));
        assert!(json.contains("New repository"));
    }

    #[test]
    fn test_update_repo_request_skip_none() {
        let req = UpdateRepoRequest {
            name: None,
            description: Some("Updated desc".to_string()),
            private: None,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("name"));
        assert!(json.contains("Updated desc"));
        assert!(!json.contains("private"));
    }

    #[test]
    fn test_update_repo_request_all_some() {
        let req = UpdateRepoRequest {
            name: Some("new-name".to_string()),
            description: Some("New desc".to_string()),
            private: Some(false),
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("new-name"));
        assert!(json.contains("New desc"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_transfer_repo_request() {
        let req = TransferRepoRequest {
            new_owner: "new-owner".to_string(),
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("new_owner"));
        assert!(json.contains("new-owner"));
    }

    #[test]
    fn test_repository_clone() {
        let repo = Repository {
            id: 1,
            name: "original".to_string(),
            full_name: "owner/original".to_string(),
            owner: Owner {
                id: 1,
                login: "owner".to_string(),
                full_name: "Owner".to_string(),
                email: "owner@test.com".to_string(),
                avatar_url: "".to_string(),
            },
            description: "".to_string(),
            private: false,
            fork: false,
            html_url: "".to_string(),
            ssh_url: "".to_string(),
            clone_url: "".to_string(),
            stars_count: 0,
            watchers_count: 0,
            forks_count: 0,
            open_issues_count: 0,
            default_branch: "main".to_string(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        
        let cloned = repo.clone();
        assert_eq!(cloned.name, "original");
    }
}

pub async fn create_repo(forgejo_url: &str, forgejo_token: &str, owner: &str, name: &str) -> Result<Repository> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    // Determine if owner is an org or user
    // For simplicity, try org first, then user
    let body = CreateRepoRequest {
        name: name.to_string(),
        description: String::new(),
        private: true,
        auto_init: true,
    };
    
    // Try creating in org first
    let org_url = client.url(&format!("/orgs/{}/repos", owner));
    let user_url = client.url("/user/repos");
    
    // First try as org repo
    let response = client
        .http()
        .post(&org_url)
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    // If org doesn't exist, try as user repo
    let response = if response.status().as_u16() == 404 {
        client
            .http()
            .post(&user_url)
            .header("Authorization", format!("token {}", client.token()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?
    } else {
        response
    };
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to create repo: {} - {}", status, body)));
    }
    
    let repo: Repository = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse repo: {}", e)))?;
    
    Ok(repo)
}

pub async fn delete_repo(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .delete(client.url(&format!("/repos/{}/{}", owner, repo)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to delete repo: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn transfer_repo(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str, new_owner: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let body = TransferRepoRequest {
        new_owner: new_owner.to_string(),
    };
    
    let response = client
        .http()
        .post(client.url(&format!("/repos/{}/{}/transfer", owner, repo)))
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to transfer repo: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn update_repo(
    forgejo_url: &str,
    forgejo_token: &str,
    owner: &str,
    repo: &str,
    private: Option<bool>,
    description: Option<&str>,
) -> Result<Repository> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let body = UpdateRepoRequest {
        name: None,
        description: description.map(|s| s.to_string()),
        private,
    };
    
    let response = client
        .http()
        .patch(client.url(&format!("/repos/{}/{}", owner, repo)))
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to update repo: {} - {}", status, body)));
    }
    
    let repo: Repository = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse repo: {}", e)))?;
    
    Ok(repo)
}