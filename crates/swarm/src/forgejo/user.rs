use crate::forgejo::ForgejoClient;
use crate::models::ForgejoUser;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateUserRequest {
    username: String,
    password: String,
    email: String,
    must_change_password: bool,
}

pub async fn create_user(
    forgejo_url: &str,
    forgejo_token: &str,
    username: &str,
    password: &str,
    email: &str,
) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let body = CreateUserRequest {
        username: username.to_string(),
        password: password.to_string(),
        email: email.to_string(),
        must_change_password: false,
    };
    
    let response = client
        .http()
        .post(client.url("/admin/users"))
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to create user: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn delete_user(
    forgejo_url: &str,
    forgejo_token: &str,
    username: &str,
) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .delete(client.url(&format!("/admin/users/{}", username)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to delete user: {} - {}", status, body)));
    }
    
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub login: String,
    pub email: String,
}

pub async fn list_users(
    forgejo_url: &str,
    forgejo_token: &str,
) -> Result<Vec<UserInfo>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url("/admin/users"))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to list users: {} - {}", status, body)));
    }
    
    let users: Vec<UserInfo> = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse users: {}", e)))?;
    
    Ok(users)
}