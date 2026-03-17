mod user;
mod repo;
mod collaborator;
mod org;

pub use user::*;
pub use repo::*;
pub use collaborator::*;
pub use org::*;

use reqwest::Client;

pub struct ForgejoClient {
    http: Client,
    base_url: String,
    token: String,
}

impl ForgejoClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.to_string(),
            token: token.to_string(),
        }
    }
    
    pub fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }
    
    pub fn token(&self) -> &str {
        &self.token
    }
    
    pub fn http(&self) -> &Client {
        &self.http
    }
}