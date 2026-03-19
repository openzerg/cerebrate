pub mod user;
pub mod repo;
pub mod collaborator;
pub mod org;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forgejo_client_new() {
        let client = ForgejoClient::new("http://localhost:3000", "test-token");
        assert_eq!(client.base_url, "http://localhost:3000");
        assert_eq!(client.token(), "test-token");
    }

    #[test]
    fn test_forgejo_client_url() {
        let client = ForgejoClient::new("http://localhost:3000", "token");
        assert_eq!(client.url("/admin/users"), "http://localhost:3000/api/v1/admin/users");
    }

    #[test]
    fn test_forgejo_client_url_with_trailing_slash() {
        let client = ForgejoClient::new("http://localhost:3000/", "token");
        assert_eq!(client.url("/repos"), "http://localhost:3000//api/v1/repos");
    }

    #[test]
    fn test_forgejo_client_url_longer_path() {
        let client = ForgejoClient::new("https://forgejo.example.com", "secret");
        assert_eq!(
            client.url("/repos/owner/repo/contents/file.txt"),
            "https://forgejo.example.com/api/v1/repos/owner/repo/contents/file.txt"
        );
    }

    #[test]
    fn test_forgejo_client_token() {
        let client = ForgejoClient::new("http://localhost", "my-api-token-123");
        assert_eq!(client.token(), "my-api-token-123");
    }
}