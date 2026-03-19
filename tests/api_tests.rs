use reqwest::Client;
use serde_json::{json, Value};

const BASE_URL: &str = "http://localhost:17531";
const AUTH: (&str, &str) = ("admin", "admin");

fn client() -> Client {
    Client::new()
}

async fn get(client: &Client, path: &str) -> Value {
    let resp = client
        .get(format!("{}{}", BASE_URL, path))
        .basic_auth(AUTH.0, Some(AUTH.1))
        .send()
        .await
        .expect("Request failed");
    resp.json().await.expect("Invalid JSON")
}

async fn post(client: &Client, path: &str, body: Value) -> Value {
    let resp = client
        .post(format!("{}{}", BASE_URL, path))
        .basic_auth(AUTH.0, Some(AUTH.1))
        .json(&body)
        .send()
        .await
        .expect("Request failed");
    resp.json().await.expect("Invalid JSON")
}

async fn post_empty(client: &Client, path: &str) -> Value {
    let resp = client
        .post(format!("{}{}", BASE_URL, path))
        .basic_auth(AUTH.0, Some(AUTH.1))
        .send()
        .await
        .expect("Request failed");
    resp.json().await.expect("Invalid JSON")
}

async fn patch(client: &Client, path: &str, body: Value) -> Value {
    let resp = client
        .patch(format!("{}{}", BASE_URL, path))
        .basic_auth(AUTH.0, Some(AUTH.1))
        .json(&body)
        .send()
        .await
        .expect("Request failed");
    resp.json().await.expect("Invalid JSON")
}

async fn delete(client: &Client, path: &str) -> Value {
    let resp = client
        .delete(format!("{}{}", BASE_URL, path))
        .basic_auth(AUTH.0, Some(AUTH.1))
        .send()
        .await
        .expect("Request failed");
    resp.json().await.expect("Invalid JSON")
}

fn assert_success(resp: &Value) {
    assert!(resp["success"].as_bool().unwrap_or(false), "Response: {:?}", resp);
}

fn assert_error(resp: &Value) {
    assert!(!resp["success"].as_bool().unwrap_or(true), "Expected error: {:?}", resp);
}

// ============ Health Tests ============

#[tokio::test]
async fn test_health() {
    let client = client();
    let resp = client.get(format!("{}/health", BASE_URL)).send().await.expect("Request failed");
    assert!(resp.status().is_success());
    let text = resp.text().await.expect("Invalid response");
    assert_eq!(text, "OK");
}

// ============ Auth Tests ============

#[tokio::test]
async fn test_auth_required() {
    let client = client();
    let resp = client
        .get(format!("{}/api/agents", BASE_URL))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(resp.status().as_u16(), 401);
}

// ============ Stats Tests ============

#[tokio::test]
async fn test_stats_summary() {
    let client = client();
    let resp = get(&client, "/api/stats/summary").await;
    assert_success(&resp);
    assert!(resp["data"]["total_agents"].is_number());
    assert!(resp["data"]["online_agents"].is_number());
    assert!(resp["data"]["enabled_agents"].is_number());
}

// ============ Agent Tests ============

#[tokio::test]
async fn test_agent_lifecycle() {
    let client = client();
    
    // Create agent
    let resp = post(&client, "/api/agents", json!({"name": "test-rust-agent"})).await;
    assert_success(&resp);
    assert_eq!(resp["data"]["name"], "test-rust-agent");
    
    // Get agent
    let resp = get(&client, "/api/agents/test-rust-agent").await;
    assert_success(&resp);
    assert_eq!(resp["data"]["name"], "test-rust-agent");
    
    // Get agent stats
    let resp = get(&client, "/api/agents/test-rust-agent/stats").await;
    assert_success(&resp);
    
    // Disable agent
    let resp = post_empty(&client, "/api/agents/test-rust-agent/disable").await;
    assert_success(&resp);
    
    // Verify disabled
    let resp = get(&client, "/api/agents/test-rust-agent").await;
    assert_success(&resp);
    assert_eq!(resp["data"]["enabled"], false);
    
    // Enable agent
    let resp = post_empty(&client, "/api/agents/test-rust-agent/enable").await;
    assert_success(&resp);
    
    // Verify enabled
    let resp = get(&client, "/api/agents/test-rust-agent").await;
    assert_success(&resp);
    assert_eq!(resp["data"]["enabled"], true);
    
    // Delete agent
    let resp = delete(&client, "/api/agents/test-rust-agent").await;
    assert_success(&resp);
    
    // Verify deleted
    let resp = get(&client, "/api/agents/test-rust-agent").await;
    assert_error(&resp);
}

#[tokio::test]
async fn test_list_agents() {
    let client = client();
    let resp = get(&client, "/api/agents").await;
    assert_success(&resp);
    assert!(resp["data"].is_array());
}

// ============ LLM Provider Tests ============

#[tokio::test]
async fn test_llm_provider_lifecycle() {
    let client = client();
    
    // List providers
    let resp = get(&client, "/api/llm/providers").await;
    assert_success(&resp);
    
    // Create provider
    let resp = post(&client, "/api/llm/providers", json!({
        "name": "test-provider-rust",
        "provider_type": "openai",
        "base_url": "https://api.test.com",
        "api_key": "test-key-123"
    })).await;
    assert_success(&resp);
    let provider_id = resp["data"]["id"].as_str().unwrap().to_string();
    
    // Get provider (via list)
    let resp = get(&client, "/api/llm/providers").await;
    assert_success(&resp);
    
    // Disable provider
    let resp = post_empty(&client, &format!("/api/llm/providers/{}/disable", provider_id)).await;
    assert_success(&resp);
    
    // Enable provider
    let resp = post_empty(&client, &format!("/api/llm/providers/{}/enable", provider_id)).await;
    assert_success(&resp);
    
    // Delete provider
    let resp = delete(&client, &format!("/api/llm/providers/{}", provider_id)).await;
    assert_success(&resp);
}

// ============ LLM API Key Tests ============

#[tokio::test]
async fn test_llm_api_key_lifecycle() {
    let client = client();
    
    // Create provider first
    let resp = post(&client, "/api/llm/providers", json!({
        "name": "key-test-provider",
        "provider_type": "custom",
        "base_url": "https://api.test.com",
        "api_key": "test-key"
    })).await;
    assert_success(&resp);
    let provider_id = resp["data"]["id"].as_str().unwrap().to_string();
    
    // List keys
    let resp = get(&client, "/api/llm/keys").await;
    assert_success(&resp);
    
    // Create key
    let resp = post(&client, "/api/llm/keys", json!({
        "name": "test-api-key",
        "provider_id": provider_id
    })).await;
    assert_success(&resp);
    let key_id = resp["data"]["id"].as_str().unwrap().to_string();
    
    // Delete key
    let resp = delete(&client, &format!("/api/llm/keys/{}", key_id)).await;
    assert_success(&resp);
    
    // Cleanup provider
    let resp = delete(&client, &format!("/api/llm/providers/{}", provider_id)).await;
    assert_success(&resp);
}

// ============ Git User Tests ============

#[tokio::test]
async fn test_git_users() {
    let client = client();
    
    // List users
    let resp = get(&client, "/api/git/users").await;
    assert_success(&resp);
}

#[tokio::test]
async fn test_git_user_create_delete() {
    let client = client();
    
    // Create user
    let resp = post(&client, "/api/git/users", json!({
        "username": "test-rust-user",
        "password": "testpass123"
    })).await;
    if resp["success"].as_bool().unwrap_or(false) {
        // Delete user
        let resp = delete(&client, "/api/git/users/test-rust-user").await;
        assert_success(&resp);
    }
}

// ============ Git Repo Tests ============

#[tokio::test]
async fn test_git_repos() {
    let client = client();
    let resp = get(&client, "/api/git/repos").await;
    assert_success(&resp);
}

#[tokio::test]
async fn test_git_repo_lifecycle() {
    let client = client();
    
    // Create repo
    let resp = post(&client, "/api/git/repos", json!({
        "name": "test-rust-repo",
        "description": "Test repository"
    })).await;
    
    if resp["success"].as_bool().unwrap_or(false) {
        let owner = resp["data"]["owner"]["login"].as_str().unwrap().to_string();
        let repo = resp["data"]["name"].as_str().unwrap().to_string();
        
        // Get repo
        let resp = get(&client, &format!("/api/git/repos/{}/{}", owner, repo)).await;
        assert_success(&resp);
        
        // Update repo
        let resp = patch(&client, &format!("/api/git/repos/{}/{}", owner, repo), json!({
            "description": "Updated description"
        })).await;
        assert_success(&resp);
        
        // Delete repo
        let resp = delete(&client, &format!("/api/git/repos/{}/{}", owner, repo)).await;
        assert_success(&resp);
    }
}

// ============ Git Collaborator Tests ============

#[tokio::test]
async fn test_git_collaborators() {
    let client = client();
    
    // Create repo
    let resp = post(&client, "/api/git/repos", json!({"name": "collab-test-repo"})).await;
    
    if resp["success"].as_bool().unwrap_or(false) {
        let owner = resp["data"]["owner"]["login"].as_str().unwrap().to_string();
        let repo = resp["data"]["name"].as_str().unwrap().to_string();
        
        // List collaborators
        let resp = get(&client, &format!("/api/git/repos/{}/{}/collaborators", owner, repo)).await;
        assert_success(&resp);
        
        // Cleanup
        let _ = delete(&client, &format!("/api/git/repos/{}/{}", owner, repo)).await;
    }
}

// ============ Git Org Tests ============

#[tokio::test]
async fn test_git_orgs() {
    let client = client();
    let resp = get(&client, "/api/git/orgs").await;
    assert_success(&resp);
}

#[tokio::test]
async fn test_git_org_lifecycle() {
    let client = client();
    
    // Create org
    let resp = post(&client, "/api/git/orgs", json!({"name": "test-rust-org"})).await;
    
    if resp["success"].as_bool().unwrap_or(false) {
        let org = resp["data"]["username"].as_str().unwrap().to_string();
        
        // List members
        let resp = get(&client, &format!("/api/git/orgs/{}/members", org)).await;
        assert_success(&resp);
        
        // Delete org
        let resp = delete(&client, &format!("/api/git/orgs/{}", org)).await;
        assert_success(&resp);
    }
}

// ============ Config Tests ============

#[tokio::test]
async fn test_config_export() {
    let client = client();
    let resp = get(&client, "/api/config/export").await;
    assert_success(&resp);
    assert!(resp["data"]["path"].is_string());
}

// ============ Apply Tests ============

#[tokio::test]
async fn test_apply() {
    let client = client();
    let resp = post_empty(&client, "/api/apply").await;
    // May fail if no NixOS, but should return a response
    assert!(resp["success"].is_boolean());
}

// ============ User Binding Tests ============

#[tokio::test]
async fn test_forgejo_user_binding() {
    let client = client();
    
    // Create agent
    let _ = post(&client, "/api/agents", json!({"name": "bind-test-agent"})).await;
    
    // Create forgejo user
    let resp = post(&client, "/api/git/users", json!({
        "username": "bind-test-user",
        "password": "testpass123"
    })).await;
    
    if resp["success"].as_bool().unwrap_or(false) {
        // Bind user to agent
        let resp = post(&client, "/api/git/users/bind", json!({
            "agent": "bind-test-agent",
            "forgejo_user": "bind-test-user"
        })).await;
        assert_success(&resp);
        
        // Verify binding
        let resp = get(&client, "/api/agents/bind-test-agent").await;
        assert_success(&resp);
        assert_eq!(resp["data"]["forgejo_username"], "bind-test-user");
        
        // Unbind user
        let resp = post(&client, "/api/git/users/unbind", json!({"agent": "bind-test-agent"})).await;
        assert_success(&resp);
        
        // Cleanup user
        let _ = delete(&client, "/api/git/users/bind-test-user").await;
    }
    
    // Cleanup agent
    let _ = delete(&client, "/api/agents/bind-test-agent").await;
}

// ============ Error Handling Tests ============

#[tokio::test]
async fn test_get_nonexistent_agent() {
    let client = client();
    let resp = get(&client, "/api/agents/nonexistent-agent-xyz").await;
    assert_error(&resp);
}

#[tokio::test]
async fn test_delete_nonexistent_agent() {
    let client = client();
    let resp = delete(&client, "/api/agents/nonexistent-agent-xyz").await;
    assert_error(&resp);
}

#[tokio::test]
async fn test_create_duplicate_agent() {
    let client = client();
    
    // Create first
    let _ = post(&client, "/api/agents", json!({"name": "duplicate-test-agent"})).await;
    
    // Try to create duplicate
    let resp = post(&client, "/api/agents", json!({"name": "duplicate-test-agent"})).await;
    assert_error(&resp);
    
    // Cleanup
    let _ = delete(&client, "/api/agents/duplicate-test-agent").await;
}