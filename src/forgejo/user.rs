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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_request_serialization() {
        let req = CreateUserRequest {
            username: "testuser".to_string(),
            password: "secret123".to_string(),
            email: "test@example.com".to_string(),
            must_change_password: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("secret123"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("must_change_password"));
    }

    #[test]
    fn test_user_info_deserialization() {
        let json = r#"{"login":"myuser","email":"myuser@example.com"}"#;
        let user: UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "myuser");
        assert_eq!(user.email, "myuser@example.com");
    }

    #[test]
    fn test_user_info_debug() {
        let user = UserInfo {
            login: "test".to_string(),
            email: "test@test.com".to_string(),
        };
        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_user_info_clone() {
        let user = UserInfo {
            login: "original".to_string(),
            email: "original@test.com".to_string(),
        };
        let cloned = user.clone();
        assert_eq!(cloned.login, "original");
        assert_eq!(cloned.email, "original@test.com");
    }
}