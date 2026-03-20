use reqwest::Client;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};
use std::time::Duration;
use once_cell::sync::Lazy;

pub static PYLON_URL: Lazy<String> = Lazy::new(|| {
    std::env::var("PYLON_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
});

pub static PYLON_GRPC_URL: Lazy<String> = Lazy::new(|| {
    std::env::var("PYLON_GRPC_URL").unwrap_or_else(|_| "http://localhost:50051".to_string())
});

pub static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "IkqX8V9dTSqTvqyBDsMt9iaKZn50rfhKyGCizUpaEcRQqgdeZkXuW1J4ZC8WYyXIA1Imar07oZeFW+nlgG4Gmw==".to_string())
        .into_bytes()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proxy {
    pub id: String,
    pub source_model: String,
    pub target_model: String,
    pub upstream: String,
    pub api_key: String,
    #[serde(default)]
    pub default_max_tokens: Option<i32>,
    #[serde(default)]
    pub default_temperature: Option<f64>,
    #[serde(default)]
    pub default_top_p: Option<f64>,
    #[serde(default)]
    pub default_top_k: Option<i32>,
    #[serde(default = "default_true")]
    pub support_streaming: bool,
    #[serde(default)]
    pub support_tools: bool,
    #[serde(default)]
    pub support_vision: bool,
    #[serde(default)]
    pub extra_headers: Option<String>,
    #[serde(default)]
    pub extra_body: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub proxy_id: String,
    pub agent_name: String,
    pub permission_level: String,
    pub granted_by: String,
    pub granted_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProxyRequest {
    pub id: String,
    pub source_model: String,
    pub target_model: String,
    pub upstream: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_tools: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_vision: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorizeRequest {
    pub agent_name: String,
    pub permission_level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevokeRequest {
    pub agent_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PylonError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Proxy not found: {0}")]
    NotFound(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PylonError>;

pub struct PylonClient {
    client: Client,
    base_url: String,
}

impl PylonClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        
        Self {
            client,
            base_url: PYLON_URL.clone(),
        }
    }

    fn create_admin_token(&self) -> String {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            iss: "cerebrate".to_string(),
            sub: "cerebrate".to_string(),
            role: "admin".to_string(),
            iat: now,
            exp: now + 3600,
        };
        
        encode(&Header::default(), &claims, &EncodingKey::from_secret(&JWT_SECRET)).unwrap()
    }

    fn create_agent_token(&self, agent_name: &str) -> String {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            iss: "cerebrate".to_string(),
            sub: agent_name.to_string(),
            role: "agent".to_string(),
            iat: now,
            exp: now + 3600,
        };
        
        encode(&Header::default(), &claims, &EncodingKey::from_secret(&JWT_SECRET)).unwrap()
    }

    pub async fn list_proxies(&self) -> Result<Vec<Proxy>> {
        let resp = self.client
            .get(&format!("{}/v1/proxies", self.base_url))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .send()
            .await?;
        
        let proxies = resp.json().await?;
        Ok(proxies)
    }

    pub async fn get_proxy(&self, id: &str) -> Result<Option<Proxy>> {
        let resp = self.client
            .get(&format!("{}/v1/proxies/{}", self.base_url, id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .send()
            .await?;
        
        if resp.status() == 404 {
            return Ok(None);
        }
        
        let proxy = resp.json().await?;
        Ok(Some(proxy))
    }

    pub async fn get_proxy_by_model(&self, model: &str) -> Result<Option<Proxy>> {
        let proxies = self.list_proxies().await?;
        Ok(proxies.into_iter().find(|p| p.source_model == model))
    }

    pub async fn create_proxy(&self, req: &CreateProxyRequest) -> Result<()> {
        let resp = self.client
            .post(&format!("{}/v1/proxies", self.base_url))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .json(req)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PylonError::Internal(format!("{}: {}", status, body)));
        }
        
        Ok(())
    }

    pub async fn update_proxy(&self, id: &str, req: &CreateProxyRequest) -> Result<()> {
        let resp = self.client
            .post(&format!("{}/v1/proxies/{}", self.base_url, id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .json(req)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PylonError::Internal(format!("{}: {}", status, body)));
        }
        
        Ok(())
    }

    pub async fn delete_proxy(&self, id: &str) -> Result<()> {
        let resp = self.client
            .delete(&format!("{}/v1/proxies/{}", self.base_url, id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PylonError::Internal(format!("{}: {}", status, body)));
        }
        
        Ok(())
    }

    pub async fn authorize_agent(&self, proxy_id: &str, agent_name: &str, level: &str) -> Result<()> {
        let req = AuthorizeRequest {
            agent_name: agent_name.to_string(),
            permission_level: Some(level.to_string()),
        };
        
        let resp = self.client
            .post(&format!("{}/v1/proxies/{}/authorize", self.base_url, proxy_id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .json(&req)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PylonError::Internal(format!("{}: {}", status, body)));
        }
        
        Ok(())
    }

    pub async fn revoke_agent(&self, proxy_id: &str, agent_name: &str) -> Result<()> {
        let req = RevokeRequest {
            agent_name: agent_name.to_string(),
        };
        
        let resp = self.client
            .post(&format!("{}/v1/proxies/{}/revoke", self.base_url, proxy_id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .json(&req)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PylonError::Internal(format!("{}: {}", status, body)));
        }
        
        Ok(())
    }

    pub async fn list_permissions(&self, proxy_id: &str) -> Result<Vec<Permission>> {
        let resp = self.client
            .get(&format!("{}/v1/proxies/{}/permissions", self.base_url, proxy_id))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .send()
            .await?;
        
        let permissions = resp.json().await?;
        Ok(permissions)
    }

    pub async fn chat_completions(&self, agent_name: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = self.client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.create_agent_token(agent_name)))
            .json(body)
            .send()
            .await?;
        
        let status = resp.status();
        if status == 401 {
            return Err(PylonError::Unauthorized);
        }
        if status == 403 {
            return Err(PylonError::Forbidden);
        }
        if status == 404 {
            return Err(PylonError::NotFound("Model not found".to_string()));
        }
        
        let result = resp.json().await?;
        Ok(result)
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let resp = self.client
            .get(&format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.create_admin_token()))
            .send()
            .await?;
        
        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelInfo>,
        }
        
        #[derive(Deserialize)]
        struct ModelInfo {
            id: String,
        }
        
        let models: ModelsResponse = resp.json().await?;
        Ok(models.data.into_iter().map(|m| m.id).collect())
    }
}

impl Default for PylonClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pylon_url_default() {
        assert_eq!(PYLON_URL.as_str(), "http://localhost:3001");
    }

    #[test]
    fn test_pylon_grpc_url_default() {
        assert_eq!(PYLON_GRPC_URL.as_str(), "http://localhost:50051");
    }

    #[test]
    fn test_create_admin_token() {
        let client = PylonClient::new();
        let token = client.create_admin_token();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_create_agent_token() {
        let client = PylonClient::new();
        let token = client.create_agent_token("test-agent");
        assert!(!token.is_empty());
    }
}