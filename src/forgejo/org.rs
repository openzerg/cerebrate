use crate::forgejo::ForgejoClient;
use crate::{Result, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default, rename = "repo_admin_change_team_access")]
    pub repo_admin_change_team_access: bool,
}

impl Organization {
    pub fn login(&self) -> &str {
        &self.username
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub id: i64,
    pub login: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
struct CreateOrgRequest {
    username: String,
    full_name: String,
    description: String,
}

pub async fn list_orgs(forgejo_url: &str, forgejo_token: &str) -> Result<Vec<Organization>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url("/admin/orgs"))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to list orgs: {} - {}", status, body)));
    }
    
    let orgs: Vec<Organization> = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse orgs: {}", e)))?;
    
    Ok(orgs)
}

pub async fn create_org(forgejo_url: &str, forgejo_token: &str, name: &str) -> Result<Organization> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let body = CreateOrgRequest {
        username: name.to_string(),
        full_name: name.to_string(),
        description: String::new(),
    };
    
    let response = client
        .http()
        .post(client.url("/orgs"))
        .header("Authorization", format!("token {}", client.token()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to create org: {} - {}", status, body)));
    }
    
    let org: Organization = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse org: {}", e)))?;
    
    Ok(org)
}

pub async fn delete_org(forgejo_url: &str, forgejo_token: &str, org: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .delete(client.url(&format!("/orgs/{}", org)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to delete org: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn list_org_members(forgejo_url: &str, forgejo_token: &str, org: &str) -> Result<Vec<OrgMember>> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .get(client.url(&format!("/orgs/{}/members", org)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to list org members: {} - {}", status, body)));
    }
    
    let members: Vec<OrgMember> = response.json().await
        .map_err(|e| Error::Config(format!("Failed to parse org members: {}", e)))?;
    
    Ok(members)
}

pub async fn add_org_member(forgejo_url: &str, forgejo_token: &str, org: &str, username: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .put(client.url(&format!("/orgs/{}/members/{}", org, username)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to add org member: {} - {}", status, body)));
    }
    
    Ok(())
}

pub async fn remove_org_member(forgejo_url: &str, forgejo_token: &str, org: &str, username: &str) -> Result<()> {
    let client = ForgejoClient::new(forgejo_url, forgejo_token);
    
    let response = client
        .http()
        .delete(client.url(&format!("/orgs/{}/members/{}", org, username)))
        .header("Authorization", format!("token {}", client.token()))
        .send()
        .await
        .map_err(|e| Error::Config(format!("Forgejo API error: {}", e)))?;
    
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Config(format!("Failed to remove org member: {} - {}", status, body)));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_deserialization() {
        let json = r#"{
            "id": 1,
            "name": "Test Org",
            "username": "testorg",
            "full_name": "Test Organization",
            "description": "A test org",
            "avatar_url": "https://example.com/avatar.png",
            "website": "https://example.com",
            "location": "Earth",
            "email": "org@example.com",
            "visibility": "public",
            "repo_admin_change_team_access": true
        }"#;
        
        let org: Organization = serde_json::from_str(json).unwrap();
        assert_eq!(org.id, 1);
        assert_eq!(org.name, "Test Org");
        assert_eq!(org.username, "testorg");
        assert!(org.repo_admin_change_team_access);
    }

    #[test]
    fn test_organization_login() {
        let org = Organization {
            id: 1,
            name: "Org".to_string(),
            username: "orguser".to_string(),
            full_name: "".to_string(),
            description: "".to_string(),
            avatar_url: "".to_string(),
            website: "".to_string(),
            location: "".to_string(),
            email: "".to_string(),
            visibility: "".to_string(),
            repo_admin_change_team_access: false,
        };
        
        assert_eq!(org.login(), "orguser");
    }

    #[test]
    fn test_organization_default_fields() {
        let json = r#"{"id": 42}"#;
        let org: Organization = serde_json::from_str(json).unwrap();
        assert_eq!(org.id, 42);
        assert_eq!(org.name, "");
        assert_eq!(org.username, "");
    }

    #[test]
    fn test_org_member_serialization() {
        let member = OrgMember {
            id: 1,
            login: "member1".to_string(),
            full_name: "Member One".to_string(),
            email: "member@example.com".to_string(),
            avatar_url: "https://example.com/avatar.png".to_string(),
        };
        
        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("member1"));
    }

    #[test]
    fn test_create_org_request() {
        let req = CreateOrgRequest {
            username: "neworg".to_string(),
            full_name: "New Organization".to_string(),
            description: "Description".to_string(),
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("neworg"));
        assert!(json.contains("New Organization"));
    }

    #[test]
    fn test_organization_clone() {
        let org = Organization {
            id: 1,
            name: "original".to_string(),
            username: "orig".to_string(),
            full_name: "".to_string(),
            description: "".to_string(),
            avatar_url: "".to_string(),
            website: "".to_string(),
            location: "".to_string(),
            email: "".to_string(),
            visibility: "".to_string(),
            repo_admin_change_team_access: false,
        };
        
        let cloned = org.clone();
        assert_eq!(cloned.name, "original");
    }
}