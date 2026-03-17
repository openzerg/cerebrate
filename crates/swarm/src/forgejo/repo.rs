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