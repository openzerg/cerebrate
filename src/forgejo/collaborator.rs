use crate::forgejo::ForgejoClient;
use crate::{Result, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub id: i64,
    pub login: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
    pub permissions: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub admin: bool,
    pub push: bool,
    pub pull: bool,
}

#[derive(Debug, Serialize)]
struct AddCollaboratorRequest {
    permission: String,
}

pub async fn list_collaborators(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str) -> Result<Vec<Collaborator>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url(&format!("/repos/{}/{}/collaborators", owner, repo)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to list collaborators: {} - {}", status, body)));
    }
    
    let collaborators: Vec<Collaborator> = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse collaborators: {}", e)))?;
    
    Ok(collaborators)
}

pub async fn add_collaborator(
    forgejo_url: &str,
    forgejo_token: &str,
    owner: &str,
    repo: &str,
    username: &str,
    permission: Option<&str>,
) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let perm = permission.unwrap_or("write");
    let body = AddCollaboratorRequest {
        permission: perm.to_string(),
    };
    
    let response = client
        .http()
        .put(client.url(&format!("/repos/{}/{}/collaborators/{}", owner, repo, username)))
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to add collaborator: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn remove_collaborator(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str, username: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .delete(client.url(&format!("/repos/{}/{}/collaborators/{}", owner, repo, username)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to remove collaborator: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn get_collaborator(forgejo_url: &str, forgejo_token: &str, owner: &str, repo: &str, username: &str) -> Result<Option<Collaborator>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url(&format!("/repos/{}/{}/collaborators/{}", owner, repo, username)))
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
        return Err(Error::Config(format!("Failed to get collaborator: {} - {}", status, body)));
    }
    
    let collaborator: Collaborator = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse collaborator: {}", e)))?;
    
    Ok(Some(collaborator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collaborator_deserialization() {
        let json = r#"{
            "id": 1,
            "login": "collab1",
            "full_name": "Collaborator One",
            "email": "collab@example.com",
            "avatar_url": "https://example.com/avatar.png",
            "permissions": {
                "admin": false,
                "push": true,
                "pull": true
            }
        }"#;
        
        let collab: Collaborator = serde_json::from_str(json).unwrap();
        assert_eq!(collab.id, 1);
        assert_eq!(collab.login, "collab1");
        assert!(!collab.permissions.admin);
        assert!(collab.permissions.push);
    }

    #[test]
    fn test_permissions_serialization() {
        let perms = Permissions {
            admin: true,
            push: true,
            pull: true,
        };
        
        let json = serde_json::to_string(&perms).unwrap();
        assert!(json.contains("admin"));
        assert!(json.contains("push"));
        assert!(json.contains("pull"));
    }

    #[test]
    fn test_collaborator_clone() {
        let collab = Collaborator {
            id: 1,
            login: "original".to_string(),
            full_name: "".to_string(),
            email: "".to_string(),
            avatar_url: "".to_string(),
            permissions: Permissions {
                admin: false,
                push: false,
                pull: true,
            },
        };
        
        let cloned = collab.clone();
        assert_eq!(cloned.login, "original");
        assert!(cloned.permissions.pull);
    }

    #[test]
    fn test_add_collaborator_request() {
        let req = AddCollaboratorRequest {
            permission: "write".to_string(),
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("write"));
    }
}