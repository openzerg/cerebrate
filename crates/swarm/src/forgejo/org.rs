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